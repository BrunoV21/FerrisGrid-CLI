use crate::sequence::{Sequence, SequenceStep};
use ferrisgrid_core::{
    ActionKind, CaptureBackend, CaptureTarget, ErrorKind, FerrisError, ImageFormat, ImageSizeLimit,
    InputBackend, InputCapabilities, PreparedAction, Result, ScreenInfo, SessionStore,
    action_summary, prepare_action, render_action_block,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ReplayRequest {
    pub source: PathBuf,
    pub output_dir: PathBuf,
    pub session: Option<String>,
    pub execute: bool,
    pub delay_ms: u64,
    pub max_actions: usize,
    pub screen_map: BTreeMap<String, String>,
    pub format: ImageFormat,
    pub grid_overlay: bool,
    pub image_size_limit: ImageSizeLimit,
}

#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub source_sequence: PathBuf,
    pub source_session: String,
    pub replay_session: Option<PathBuf>,
    pub actions: usize,
    pub checkpoints: usize,
    pub external_state_warnings: usize,
    pub executed: bool,
}

#[derive(Debug)]
struct PreparedStep {
    source: SequenceStep,
    prepared: PreparedAction,
}

pub fn replay(
    request: ReplayRequest,
    capture: Arc<dyn CaptureBackend>,
    input: Arc<dyn InputBackend>,
) -> Result<ReplayResult> {
    validate_request(&request)?;
    let sequence_path = resolve_sequence_path(&request.source);
    let sequence = Sequence::read(&sequence_path)?;
    if sequence.steps.len() > request.max_actions {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            format!(
                "sequence contains {} actions but --max-actions is {}",
                sequence.steps.len(),
                request.max_actions
            ),
        ));
    }
    let screens = capture.list_screens()?;
    let mapping = build_screen_mapping(&sequence, &screens, &request.screen_map)?;
    let prepared = preflight(&sequence, &screens, &mapping)?;
    preflight_input_capabilities(&prepared, input.capabilities())?;
    let checkpoints = prepared
        .iter()
        .filter(|step| step.source.checkpoint.is_some())
        .count();
    let warnings = prepared
        .iter()
        .filter(|step| step.source.external_state.is_some())
        .count();
    if !request.execute {
        return Ok(ReplayResult {
            source_sequence: sequence_path,
            source_session: sequence.source_session,
            replay_session: None,
            actions: prepared.len(),
            checkpoints,
            external_state_warnings: warnings,
            executed: false,
        });
    }

    let store = SessionStore::new(&request.output_dir);
    let session_dir = store.create_exclusive_session(request.session.as_deref())?;
    write_replay_manifest(
        &session_dir,
        &sequence.source_session,
        "running",
        prepared.len(),
        0,
        0,
    )?;
    let mut frame = store.next_step(&session_dir)?;
    let mut completed = 0;
    let mut frames_written = 0;
    let initial_frame_dir = store.frame_dir(&session_dir, frame).map_err(|error| {
        mark_replay_failed(
            &store,
            &session_dir,
            &sequence.source_session,
            prepared.len(),
            completed,
            frames_written,
            error,
        )
    })?;
    capture
        .capture(
            CaptureTarget::All,
            &initial_frame_dir,
            &request.format,
            request.grid_overlay,
            request.image_size_limit,
        )
        .map_err(|error| {
            mark_replay_failed(
                &store,
                &session_dir,
                &sequence.source_session,
                prepared.len(),
                completed,
                frames_written,
                error,
            )
        })?;
    frames_written += 1;
    frame += 1;
    for (index, step) in prepared.iter().enumerate() {
        input.execute(&step.prepared.native).map_err(|error| {
            mark_replay_failed(
                &store,
                &session_dir,
                &sequence.source_session,
                prepared.len(),
                completed,
                frames_written,
                error,
            )
        })?;
        completed += 1;
        if request.delay_ms > 0 {
            thread::sleep(Duration::from_millis(request.delay_ms));
        }
        if step.source.checkpoint.is_some() {
            let target = step
                .prepared
                .target_screen_id
                .as_ref()
                .map(|screen| CaptureTarget::Screen(screen.clone()))
                .unwrap_or(CaptureTarget::All);
            let frame_dir = store.frame_dir(&session_dir, frame).map_err(|error| {
                mark_replay_failed(
                    &store,
                    &session_dir,
                    &sequence.source_session,
                    prepared.len(),
                    completed,
                    frames_written,
                    error,
                )
            })?;
            capture
                .capture(
                    target,
                    &frame_dir,
                    &request.format,
                    request.grid_overlay,
                    request.image_size_limit,
                )
                .map_err(|error| {
                    mark_replay_failed(
                        &store,
                        &session_dir,
                        &sequence.source_session,
                        prepared.len(),
                        completed,
                        frames_written,
                        error,
                    )
                })?;
            frames_written += 1;
            frame += 1;
        }
        let markdown = render_action_block(&step.prepared.action, None);
        store
            .write_action_files(
                &session_dir,
                index as u32 + 1,
                &markdown,
                &action_summary(&step.prepared.action),
                "replayed",
            )
            .map_err(|error| {
                mark_replay_failed(
                    &store,
                    &session_dir,
                    &sequence.source_session,
                    prepared.len(),
                    completed,
                    frames_written,
                    error,
                )
            })?;
        store
            .append_event(
                &session_dir,
                format!(
                    "{} sequence_action_replayed source_step={} action={}",
                    unix_millis(),
                    step.source.number,
                    action_summary(&step.prepared.action)
                ),
            )
            .map_err(|error| {
                mark_replay_failed(
                    &store,
                    &session_dir,
                    &sequence.source_session,
                    prepared.len(),
                    completed,
                    frames_written,
                    error,
                )
            })?;
    }
    write_replay_manifest(
        &session_dir,
        &sequence.source_session,
        "complete",
        prepared.len(),
        completed,
        frames_written,
    )?;
    Ok(ReplayResult {
        source_sequence: sequence_path,
        source_session: sequence.source_session,
        replay_session: Some(session_dir),
        actions: prepared.len(),
        checkpoints,
        external_state_warnings: warnings,
        executed: true,
    })
}

fn preflight(
    sequence: &Sequence,
    screens: &[ScreenInfo],
    mapping: &BTreeMap<String, String>,
) -> Result<Vec<PreparedStep>> {
    let mut prepared = Vec::with_capacity(sequence.steps.len());
    for step in &sequence.steps {
        if step.redacted || step.omitted {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                format!(
                    "sequence step {} has {} typed text and cannot be replayed; record with --text-mode plain",
                    step.number,
                    if step.redacted { "redacted" } else { "omitted" }
                ),
            ));
        }
        let mut action = step.action.clone();
        if let Some(kind) = action.kind.take() {
            action.kind = Some(remap_action(kind, mapping)?);
        }
        let prepared_action = prepare_action(&action, screens, None).map_err(|error| {
            FerrisError::new(
                error.kind,
                format!(
                    "sequence step {} failed preflight: {}",
                    step.number, error.message
                ),
            )
        })?;
        prepared.push(PreparedStep {
            source: step.clone(),
            prepared: prepared_action,
        });
    }
    Ok(prepared)
}

fn preflight_input_capabilities(
    steps: &[PreparedStep],
    capabilities: InputCapabilities,
) -> Result<()> {
    for step in steps {
        let (needs_mouse, needs_keyboard) = match &step.prepared.action {
            ActionKind::Click { .. }
            | ActionKind::DoubleClick { .. }
            | ActionKind::RightClick { .. }
            | ActionKind::MoveMouse { .. }
            | ActionKind::Drag { .. }
            | ActionKind::Scroll { .. } => (true, false),
            ActionKind::Type { .. } | ActionKind::PressKey { .. } | ActionKind::Hotkey { .. } => {
                (false, true)
            }
            ActionKind::Wait { .. } => (false, false),
        };
        if needs_mouse && !capabilities.can_mouse {
            return Err(FerrisError::new(
                ErrorKind::Platform,
                format!(
                    "sequence step {} requires mouse input but the selected backend cannot emit it",
                    step.source.number
                ),
            ));
        }
        if needs_keyboard && !capabilities.can_keyboard {
            return Err(FerrisError::new(
                ErrorKind::Platform,
                format!(
                    "sequence step {} requires keyboard input but the selected backend cannot emit it",
                    step.source.number
                ),
            ));
        }
    }
    Ok(())
}

fn remap_action(action: ActionKind, mapping: &BTreeMap<String, String>) -> Result<ActionKind> {
    let mapped = action
        .screen_id()
        .map(|screen_id| {
            mapping.get(screen_id).cloned().ok_or_else(|| {
                FerrisError::new(
                    ErrorKind::Coordinate,
                    format!("recorded screen {screen_id} has no current display mapping"),
                )
            })
        })
        .transpose()?;
    Ok(action.with_screen_id(mapped))
}

fn build_screen_mapping(
    sequence: &Sequence,
    current: &[ScreenInfo],
    explicit: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut mapping = BTreeMap::new();
    for recorded in &sequence.screens {
        let current_id = if let Some(explicit) = explicit.get(&recorded.screen_id) {
            if current.iter().all(|screen| &screen.screen_id != explicit) {
                return Err(FerrisError::new(
                    ErrorKind::Coordinate,
                    format!(
                        "explicit screen mapping {}={} targets an unknown current screen",
                        recorded.screen_id, explicit
                    ),
                ));
            }
            explicit.clone()
        } else if let Some(screen) = current
            .iter()
            .find(|screen| screen.display_fingerprint == recorded.fingerprint)
        {
            screen.screen_id.clone()
        } else if let Some(screen) = current
            .iter()
            .find(|screen| screen.screen_id == recorded.screen_id)
        {
            screen.screen_id.clone()
        } else {
            return Err(FerrisError::new(
                ErrorKind::Coordinate,
                format!(
                    "recorded screen {} fingerprint={} is not active; use --map-screen recorded=current",
                    recorded.screen_id, recorded.fingerprint
                ),
            ));
        };
        mapping.insert(recorded.screen_id.clone(), current_id);
    }
    Ok(mapping)
}

fn resolve_sequence_path(source: &Path) -> PathBuf {
    if source.is_dir() {
        source.join("sequences").join("sequence.md")
    } else {
        source.to_path_buf()
    }
}

fn validate_request(request: &ReplayRequest) -> Result<()> {
    if request.max_actions == 0 || request.max_actions > 1000 {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "--max-actions must be within 1..1000",
        ));
    }
    if request.delay_ms > 30_000 {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "--delay-ms must not exceed 30000",
        ));
    }
    Ok(())
}

fn write_replay_manifest(
    session_dir: &Path,
    source_session: &str,
    status: &str,
    actions: usize,
    completed_actions: usize,
    frames: u32,
) -> Result<()> {
    fs::write(
        session_dir.join("manifest.md"),
        format!(
            "## FerrisGrid Session\n- schema_version: 2\n- session_id: {}\n- session_mode: replay\n- source_session: {source_session}\n- updated_at_unix_ms: {}\n- status: {status}\n- actions: {actions}\n- completed_actions: {completed_actions}\n- frames: {frames}\n",
            session_dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("replay"),
            unix_millis()
        ),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn mark_replay_failed(
    store: &SessionStore,
    session_dir: &Path,
    source_session: &str,
    actions: usize,
    completed_actions: usize,
    frames: u32,
    error: FerrisError,
) -> FerrisError {
    let _ = write_replay_manifest(
        session_dir,
        source_session,
        "failed",
        actions,
        completed_actions,
        frames,
    );
    let _ = store.append_event(
        session_dir,
        format!(
            "{} sequence_replay_failed completed_actions={} reason={}",
            unix_millis(),
            completed_actions,
            error.message
        ),
    );
    error
}

pub fn render_replay_result(result: &ReplayResult) -> String {
    let session = result
        .replay_session
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "## FerrisGrid Replay\n- mode: {}\n- source_sequence: {}\n- source_session: {}\n- replay_session: {}\n- actions: {}\n- checkpoints: {}\n- external_state_warnings: {}\n- result: {}\n",
        if result.executed {
            "execute"
        } else {
            "dry_run"
        },
        result.source_sequence.display(),
        result.source_session,
        session,
        result.actions,
        result.checkpoints,
        result.external_state_warnings,
        if result.executed {
            "success"
        } else {
            "validated"
        }
    )
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reducer::CheckpointReason;
    use crate::sequence::{SequenceScreen, TextMode};
    use ferrisgrid_capture::FakeCaptureBackend;
    use ferrisgrid_core::{ActionStatus, AgentAction, InputExecution, MouseButton, NativeAction};

    fn current_screen() -> ScreenInfo {
        ScreenInfo {
            screen_id: "screen-9".to_string(),
            display_fingerprint: "display-a".to_string(),
            name: "Current".to_string(),
            is_primary: true,
            origin_x: 0,
            origin_y: 0,
            logical_width: 1000,
            logical_height: 1000,
            native_width: 2000,
            native_height: 2000,
            scale_factor: 2.0,
        }
    }

    #[test]
    fn fingerprint_mapping_preflights_recorded_pointer_action() {
        let sequence = Sequence {
            source_session: "source".to_string(),
            text_mode: TextMode::Plain,
            screens: vec![SequenceScreen {
                screen_id: "screen-1".to_string(),
                fingerprint: "display-a".to_string(),
                logical_width: 1000,
                logical_height: 1000,
            }],
            steps: vec![SequenceStep {
                number: 1,
                occurred_at_ms: 1,
                started_at_ms: 1,
                action: AgentAction {
                    status: ActionStatus::Action,
                    kind: Some(ActionKind::Click {
                        screen_id: Some("screen-1".to_string()),
                        x: 500,
                        y: 500,
                        button: MouseButton::Left,
                    }),
                    wait_after_ms: None,
                    confidence: None,
                    reason: None,
                },
                checkpoint: Some(CheckpointReason::Click),
                before_frame: None,
                after_frame: None,
                redacted: false,
                omitted: false,
                external_state: None,
            }],
        };
        let screens = vec![current_screen()];
        let mapping = build_screen_mapping(&sequence, &screens, &BTreeMap::new()).unwrap();
        let prepared = preflight(&sequence, &screens, &mapping).unwrap();
        assert_eq!(
            prepared[0].prepared.target_screen_id.as_deref(),
            Some("screen-9")
        );
        let error = preflight_input_capabilities(
            &prepared,
            InputCapabilities {
                can_mouse: false,
                can_keyboard: true,
            },
        )
        .unwrap_err();
        assert!(error.message.contains("requires mouse input"));
    }

    #[test]
    fn redacted_text_fails_before_preparing_any_action() {
        let sequence = Sequence {
            source_session: "source".to_string(),
            text_mode: TextMode::Redacted,
            screens: vec![],
            steps: vec![SequenceStep {
                number: 1,
                occurred_at_ms: 1,
                started_at_ms: 1,
                action: AgentAction {
                    status: ActionStatus::Action,
                    kind: Some(ActionKind::Type {
                        text: "secret".to_string(),
                    }),
                    wait_after_ms: None,
                    confidence: None,
                    reason: None,
                },
                checkpoint: None,
                before_frame: None,
                after_frame: None,
                redacted: true,
                omitted: false,
                external_state: None,
            }],
        };
        let error = preflight(&sequence, &[current_screen()], &BTreeMap::new()).unwrap_err();
        assert!(error.message.contains("--text-mode plain"));
    }

    struct FailingInput;

    impl InputBackend for FailingInput {
        fn name(&self) -> &'static str {
            "failing"
        }

        fn capabilities(&self) -> InputCapabilities {
            InputCapabilities {
                can_mouse: true,
                can_keyboard: true,
            }
        }

        fn execute(&self, _action: &NativeAction) -> Result<InputExecution> {
            Err(FerrisError::new(
                ErrorKind::Execution,
                "intentional input failure",
            ))
        }
    }

    #[test]
    fn live_failure_finalizes_replay_manifest() {
        let root = std::env::temp_dir().join(format!(
            "ferrisgrid-replay-failure-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        let source = root.join("sequence.md");
        Sequence {
            source_session: "source".to_string(),
            text_mode: TextMode::Plain,
            screens: vec![SequenceScreen {
                screen_id: "screen-1".to_string(),
                fingerprint: "fake-primary".to_string(),
                logical_width: 1512,
                logical_height: 982,
            }],
            steps: vec![SequenceStep {
                number: 1,
                occurred_at_ms: 1,
                started_at_ms: 1,
                action: AgentAction {
                    status: ActionStatus::Action,
                    kind: Some(ActionKind::Click {
                        screen_id: Some("screen-1".to_string()),
                        x: 500,
                        y: 500,
                        button: MouseButton::Left,
                    }),
                    wait_after_ms: None,
                    confidence: None,
                    reason: None,
                },
                checkpoint: Some(CheckpointReason::Click),
                before_frame: None,
                after_frame: None,
                redacted: false,
                omitted: false,
                external_state: None,
            }],
        }
        .write_atomic(&source)
        .unwrap();
        let output_dir = root.join("output");
        let error = replay(
            ReplayRequest {
                source,
                output_dir: output_dir.clone(),
                session: Some("failed-replay".to_string()),
                execute: true,
                delay_ms: 0,
                max_actions: 25,
                screen_map: BTreeMap::new(),
                format: ImageFormat::Jpg,
                grid_overlay: false,
                image_size_limit: ImageSizeLimit::FixedMaxEdge(800),
            },
            Arc::new(FakeCaptureBackend::new()),
            Arc::new(FailingInput),
        )
        .unwrap_err();
        assert_eq!(error.message, "intentional input failure");
        let manifest = fs::read_to_string(
            output_dir
                .join("sessions/failed-replay")
                .join("manifest.md"),
        )
        .unwrap();
        assert!(manifest.contains("- status: failed"));
        assert!(manifest.contains("- completed_actions: 0"));
        let _ = fs::remove_dir_all(root);
    }
}
