use crate::reducer::{ControlEvent, RawInputEvent, SemanticReducer, SemanticStep};
use crate::rolling::RollingCapture;
use crate::sequence::{Sequence, SequenceScreen, SequenceStep, TextMode};
use ferrisgrid_core::{
    ActionKind, ActionStatus, AgentAction, CaptureBackend, ErrorKind, FerrisError, ImageFormat,
    ImageSizeLimit, Result, SessionStore, action_summary, render_action_block,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const EVENT_QUEUE_CAPACITY: usize = 4096;
static CTRL_C_TARGET: OnceLock<Mutex<Option<SyncSender<RawInputEvent>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub struct EventSourceCapabilities {
    pub mouse: bool,
    pub keyboard: bool,
    pub global_controls: bool,
}

pub trait EventSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> EventSourceCapabilities;
    fn run(&self, sender: SyncSender<RawInputEvent>, stop: Arc<AtomicBool>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct RecordRequest {
    pub output_dir: PathBuf,
    pub session: Option<String>,
    pub text_mode: TextMode,
    pub format: ImageFormat,
    pub image_size_limit: ImageSizeLimit,
    pub fps: u32,
    pub settle_ms: u64,
    pub countdown_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RecordResult {
    pub session_dir: PathBuf,
    pub sequence_path: PathBuf,
    pub recording_path: PathBuf,
    pub actions: usize,
    pub frames: u32,
    pub replayable: bool,
    pub event_source: String,
}

pub fn record(
    request: RecordRequest,
    capture: Arc<dyn CaptureBackend>,
    event_source: Box<dyn EventSource>,
) -> Result<RecordResult> {
    validate_record_request(&request)?;
    let store = SessionStore::new(&request.output_dir);
    let session_dir = store.create_exclusive_session(request.session.as_deref())?;
    let session_id = session_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("recording")
        .to_string();
    let screens = capture.list_screens()?;
    if screens.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "recording requires at least one active screen",
        ));
    }
    let sequences_dir = session_dir.join("sequences");
    fs::create_dir_all(&sequences_dir)?;
    let sequence_path = sequences_dir.join("sequence.md");
    let recording_path = sequences_dir.join("recording.md");
    let mut sequence = Sequence {
        source_session: session_id.clone(),
        text_mode: request.text_mode,
        screens: screens.iter().map(SequenceScreen::from).collect(),
        steps: Vec::new(),
    };
    write_manifest(
        &session_dir,
        &session_id,
        "recording",
        request.text_mode,
        "running",
        0,
        0,
        false,
    )?;
    write_recording_summary(
        &recording_path,
        &session_id,
        event_source.name(),
        &request,
        "running",
        0,
        0,
    )?;

    let mut rolling = match RollingCapture::start(
        capture,
        request.format.clone(),
        request.image_size_limit,
        request.fps,
        &screens,
    ) {
        Ok(rolling) => rolling,
        Err(error) => {
            mark_record_failed(
                &store,
                &session_dir,
                &session_id,
                event_source.name(),
                &request,
                0,
                0,
                &error,
            )?;
            return Err(error);
        }
    };
    let initial = match rolling.wait_initial(Duration::from_secs(10)) {
        Ok(initial) => initial,
        Err(error) => {
            rolling.stop();
            mark_record_failed(
                &store,
                &session_dir,
                &session_id,
                event_source.name(),
                &request,
                0,
                0,
                &error,
            )?;
            return Err(error);
        }
    };
    let mut next_frame = match store.next_step(&session_dir) {
        Ok(frame) => frame,
        Err(error) => {
            rolling.stop();
            mark_record_failed(
                &store,
                &session_dir,
                &session_id,
                event_source.name(),
                &request,
                0,
                0,
                &error,
            )?;
            return Err(error);
        }
    };
    if let Err(error) = initial.persist(&store, &session_dir, next_frame) {
        rolling.stop();
        mark_record_failed(
            &store,
            &session_dir,
            &session_id,
            event_source.name(),
            &request,
            0,
            0,
            &error,
        )?;
        return Err(error);
    }
    if let Err(error) = store.append_event(
        &session_dir,
        format!(
            "{} recording_started event_source={} initial_frame={:06}",
            unix_millis(),
            event_source.name(),
            next_frame
        ),
    ) {
        rolling.stop();
        mark_record_failed(
            &store,
            &session_dir,
            &session_id,
            event_source.name(),
            &request,
            0,
            1,
            &error,
        )?;
        return Err(error);
    }
    next_frame += 1;

    let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
    if let Err(error) = install_ctrl_c_target(sender.clone()) {
        rolling.stop();
        mark_record_failed(
            &store,
            &session_dir,
            &session_id,
            event_source.name(),
            &request,
            0,
            1,
            &error,
        )?;
        return Err(error);
    }
    if request.countdown_ms > 0 {
        thread::sleep(Duration::from_millis(request.countdown_ms));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let source_stop = stop.clone();
    let (source_result_sender, source_result_receiver) = mpsc::channel();
    let source_name = event_source.name().to_string();
    let source_thread = thread::spawn(move || {
        let result = event_source.run(sender.clone(), source_stop);
        let _ = source_result_sender.send(result);
        let _ = sender.send(RawInputEvent::Control {
            at_ms: unix_millis(),
            control: ControlEvent::Stop,
        });
    });

    let processing = process_events(
        &request,
        &store,
        &session_dir,
        &sequence_path,
        &mut sequence,
        &screens,
        &mut rolling,
        &receiver,
        &stop,
        &mut next_frame,
    );
    stop.store(true, Ordering::Relaxed);
    clear_ctrl_c_target();
    let _ = source_thread.join();
    let source_result = source_result_receiver.try_recv().unwrap_or(Ok(()));
    if let Err(error) = processing.and(source_result) {
        rolling.stop();
        mark_record_failed(
            &store,
            &session_dir,
            &session_id,
            &source_name,
            &request,
            sequence.steps.len(),
            next_frame.saturating_sub(1),
            &error,
        )?;
        return Err(error);
    }

    let final_snapshot = rolling.latest().unwrap_or_else(|_| initial.clone());
    final_snapshot.persist(&store, &session_dir, next_frame)?;
    next_frame += 1;
    rolling.stop();
    sequence.write_atomic(&sequence_path)?;
    let replayable = sequence.replayable();
    write_manifest(
        &session_dir,
        &session_id,
        "recording",
        request.text_mode,
        "complete",
        sequence.steps.len(),
        next_frame.saturating_sub(1),
        replayable,
    )?;
    write_recording_summary(
        &recording_path,
        &session_id,
        &source_name,
        &request,
        "complete",
        sequence.steps.len(),
        next_frame.saturating_sub(1),
    )?;
    store.append_event(
        &session_dir,
        format!(
            "{} recording_stopped actions={} frames={} replayable={replayable}",
            unix_millis(),
            sequence.steps.len(),
            next_frame.saturating_sub(1)
        ),
    )?;
    Ok(RecordResult {
        session_dir,
        sequence_path,
        recording_path,
        actions: sequence.steps.len(),
        frames: next_frame.saturating_sub(1),
        replayable,
        event_source: source_name,
    })
}

#[allow(clippy::too_many_arguments)]
fn process_events(
    request: &RecordRequest,
    store: &SessionStore,
    session_dir: &Path,
    sequence_path: &Path,
    sequence: &mut Sequence,
    screens: &[ferrisgrid_core::ScreenInfo],
    rolling: &mut RollingCapture,
    receiver: &Receiver<RawInputEvent>,
    stop: &Arc<AtomicBool>,
    next_frame: &mut u32,
) -> Result<()> {
    let mut reducer = SemanticReducer::new(screens.to_vec());
    loop {
        let event = match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => RawInputEvent::Tick {
                at_ms: unix_millis(),
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => RawInputEvent::Control {
                at_ms: unix_millis(),
                control: ControlEvent::Stop,
            },
        };
        let should_stop = matches!(
            &event,
            RawInputEvent::Control {
                control: ControlEvent::Stop,
                ..
            }
        );
        let should_pause = matches!(
            &event,
            RawInputEvent::Control {
                control: ControlEvent::Pause,
                ..
            }
        );
        if matches!(&event, RawInputEvent::Tick { .. }) {
            rolling.check_health()?;
        }
        if let RawInputEvent::Control {
            control: ControlEvent::Resume,
            ..
        } = &event
        {
            rolling.set_paused(false);
            rolling.wait_initial(Duration::from_secs(5))?;
        }
        let steps = reducer.push(event)?;
        persist_steps(
            request,
            store,
            session_dir,
            sequence_path,
            sequence,
            rolling,
            steps,
            next_frame,
        )?;
        if should_pause {
            rolling.set_paused(true);
        }
        if should_stop {
            stop.store(true, Ordering::Relaxed);
            break;
        }
    }
    let final_steps = reducer.finish(unix_millis())?;
    persist_steps(
        request,
        store,
        session_dir,
        sequence_path,
        sequence,
        rolling,
        final_steps,
        next_frame,
    )
}

#[allow(clippy::too_many_arguments)]
fn persist_steps(
    request: &RecordRequest,
    store: &SessionStore,
    session_dir: &Path,
    sequence_path: &Path,
    sequence: &mut Sequence,
    rolling: &RollingCapture,
    steps: Vec<SemanticStep>,
    next_frame: &mut u32,
) -> Result<()> {
    for semantic in steps {
        let mut before_frame = None;
        let mut after_frame = None;
        if semantic.checkpoint.is_some() {
            let before = rolling.at_or_before(semantic.started_at_ms)?;
            before.persist(store, session_dir, *next_frame)?;
            before_frame = Some(*next_frame);
            *next_frame += 1;
            let target = semantic.occurred_at_ms.saturating_add(request.settle_ms);
            let after = rolling.wait_after(target, Duration::from_secs(5))?;
            after.persist(store, session_dir, *next_frame)?;
            after_frame = Some(*next_frame);
            *next_frame += 1;
        }
        let is_type = matches!(semantic.action, ActionKind::Type { .. });
        let redacted = is_type && request.text_mode == TextMode::Redacted;
        let omitted = is_type && request.text_mode == TextMode::Off;
        let action = AgentAction {
            status: ActionStatus::Action,
            kind: Some(semantic.action.clone()),
            wait_after_ms: None,
            confidence: None,
            reason: Some("recorded human input".to_string()),
        };
        let step_number = sequence.steps.len() as u32 + 1;
        let request_text = if redacted {
            "status: action\naction: type\ntext: <redacted>".to_string()
        } else if omitted {
            "status: action\naction: type\ntext: <omitted>".to_string()
        } else {
            render_action_block(&semantic.action, None)
        };
        store.write_action_files(
            session_dir,
            step_number,
            &request_text,
            &format!(
                "source=human checkpoint={} before_frame={} after_frame={}",
                semantic
                    .checkpoint
                    .map(|value| value.as_str())
                    .unwrap_or("none"),
                before_frame
                    .map(|value| format!("{value:06}"))
                    .unwrap_or_else(|| "none".to_string()),
                after_frame
                    .map(|value| format!("{value:06}"))
                    .unwrap_or_else(|| "none".to_string())
            ),
            "recorded",
        )?;
        store.append_event(
            session_dir,
            format!(
                "{} human_action_recorded step={} action={} checkpoint={}",
                unix_millis(),
                step_number,
                action_summary(&semantic.action),
                semantic
                    .checkpoint
                    .map(|value| value.as_str())
                    .unwrap_or("none")
            ),
        )?;
        sequence.steps.push(SequenceStep {
            number: step_number,
            occurred_at_ms: semantic.occurred_at_ms,
            started_at_ms: semantic.started_at_ms,
            action,
            checkpoint: semantic.checkpoint,
            before_frame,
            after_frame,
            redacted,
            omitted,
            external_state: semantic.external_state,
        });
        sequence.write_atomic(sequence_path)?;
    }
    Ok(())
}

fn validate_record_request(request: &RecordRequest) -> Result<()> {
    if request.fps == 0 || request.fps > 30 {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "--fps must be within 1..30",
        ));
    }
    if request.settle_ms > 30_000 {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "--settle-ms must not exceed 30000",
        ));
    }
    Ok(())
}

fn install_ctrl_c_target(sender: SyncSender<RawInputEvent>) -> Result<()> {
    let target = CTRL_C_TARGET.get_or_init(|| Mutex::new(None));
    if CTRL_C_TARGET.get().is_some() {
        static HANDLER_INSTALLED: OnceLock<()> = OnceLock::new();
        if HANDLER_INSTALLED.get().is_none() {
            ctrlc::set_handler(|| {
                if let Some(target) = CTRL_C_TARGET.get()
                    && let Ok(guard) = target.lock()
                    && let Some(sender) = guard.as_ref()
                {
                    let _ = sender.try_send(RawInputEvent::Control {
                        at_ms: unix_millis(),
                        control: ControlEvent::Stop,
                    });
                }
            })
            .map_err(|error| {
                FerrisError::new(
                    ErrorKind::Platform,
                    format!("failed to install Ctrl+C handler: {error}"),
                )
            })?;
            let _ = HANDLER_INSTALLED.set(());
        }
    }
    *target
        .lock()
        .map_err(|_| FerrisError::new(ErrorKind::Platform, "Ctrl+C handler lock was poisoned"))? =
        Some(sender);
    Ok(())
}

fn clear_ctrl_c_target() {
    if let Some(target) = CTRL_C_TARGET.get()
        && let Ok(mut target) = target.lock()
    {
        *target = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn write_manifest(
    session_dir: &Path,
    session_id: &str,
    mode: &str,
    text_mode: TextMode,
    status: &str,
    actions: usize,
    frames: u32,
    replayable: bool,
) -> Result<()> {
    fs::write(
        session_dir.join("manifest.md"),
        format!(
            "## FerrisGrid Session\n- schema_version: 2\n- session_id: {session_id}\n- session_mode: {mode}\n- created_or_updated_at_unix_ms: {}\n- text_mode: {}\n- status: {status}\n- actions: {actions}\n- frames: {frames}\n- replayable: {replayable}\n",
            unix_millis(),
            text_mode.as_str()
        ),
    )?;
    Ok(())
}

fn write_recording_summary(
    path: &Path,
    session_id: &str,
    event_source: &str,
    request: &RecordRequest,
    status: &str,
    actions: usize,
    frames: u32,
) -> Result<()> {
    fs::write(
        path,
        format!(
            "## FerrisGrid Recording\n- session_id: {session_id}\n- status: {status}\n- event_source: {event_source}\n- text_mode: {}\n- rolling_fps: {}\n- settle_ms: {}\n- actions: {actions}\n- frames: {frames}\n- privacy: screenshots can contain visible sensitive text even when typed payloads are redacted\n",
            request.text_mode.as_str(),
            request.fps,
            request.settle_ms
        ),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn mark_record_failed(
    store: &SessionStore,
    session_dir: &Path,
    session_id: &str,
    event_source: &str,
    request: &RecordRequest,
    actions: usize,
    frames: u32,
    error: &FerrisError,
) -> Result<()> {
    write_manifest(
        session_dir,
        session_id,
        "recording",
        request.text_mode,
        "failed",
        actions,
        frames,
        false,
    )?;
    write_recording_summary(
        &session_dir.join("sequences/recording.md"),
        session_id,
        event_source,
        request,
        "failed",
        actions,
        frames,
    )?;
    store.append_event(
        session_dir,
        format!(
            "{} recording_failed reason={}",
            unix_millis(),
            error.message
        ),
    )
}

pub struct FakeEventSource {
    events: Vec<RawInputEvent>,
}

impl FakeEventSource {
    pub fn new(events: Vec<RawInputEvent>) -> Self {
        Self { events }
    }

    pub fn demonstration() -> Self {
        let now = unix_millis().saturating_add(100);
        Self::new(vec![
            RawInputEvent::MouseDown {
                at_ms: now,
                x: 500,
                y: 500,
                button: ferrisgrid_core::MouseButton::Left,
                click_count: 1,
            },
            RawInputEvent::MouseUp {
                at_ms: now + 30,
                x: 500,
                y: 500,
                button: ferrisgrid_core::MouseButton::Left,
                click_count: 1,
            },
            RawInputEvent::Tick { at_ms: now + 600 },
            RawInputEvent::Control {
                at_ms: now + 700,
                control: ControlEvent::Stop,
            },
        ])
    }
}

impl EventSource for FakeEventSource {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn capabilities(&self) -> EventSourceCapabilities {
        EventSourceCapabilities {
            mouse: true,
            keyboard: true,
            global_controls: true,
        }
    }

    fn run(&self, sender: SyncSender<RawInputEvent>, stop: Arc<AtomicBool>) -> Result<()> {
        for event in &self.events {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            sender.try_send(event.clone()).map_err(|_| {
                FerrisError::new(ErrorKind::Execution, "recording event queue overflowed")
            })?;
        }
        Ok(())
    }
}

pub fn render_record_result(result: &RecordResult) -> String {
    format!(
        "## FerrisGrid Record\n- session: {}\n- event_source: {}\n- actions: {}\n- frames: {}\n- replayable: {}\n- sequence: {}\n- recording: {}\n",
        result.session_dir.display(),
        result.event_source,
        result.actions,
        result.frames,
        result.replayable,
        result.sequence_path.display(),
        result.recording_path.display()
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
    use ferrisgrid_capture::FakeCaptureBackend;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn fake_recording_writes_action_sequence_and_smart_frames() {
        let output_dir = temp_output_dir("fake-record");
        let request = RecordRequest {
            output_dir: output_dir.clone(),
            session: Some("demo".to_string()),
            text_mode: TextMode::Redacted,
            format: ImageFormat::Jpg,
            image_size_limit: ImageSizeLimit::FixedMaxEdge(800),
            fps: 30,
            settle_ms: 0,
            countdown_ms: 0,
        };
        let result = record(
            request.clone(),
            Arc::new(FakeCaptureBackend::new()),
            Box::new(FakeEventSource::demonstration()),
        )
        .unwrap();
        assert_eq!(result.actions, 1);
        assert!(result.frames >= 4);
        assert!(result.replayable);
        let sequence = Sequence::read(&result.sequence_path).unwrap();
        assert_eq!(sequence.steps.len(), 1);
        assert!(sequence.steps[0].before_frame.is_some());
        assert!(sequence.steps[0].after_frame.is_some());
        assert!(result.session_dir.join("actions/000001.md").exists());
        let error = record(
            request,
            Arc::new(FakeCaptureBackend::new()),
            Box::new(FakeEventSource::demonstration()),
        )
        .unwrap_err();
        assert!(error.message.contains("will not be overwritten"));
        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn redacted_recording_does_not_persist_typed_payload() {
        let output_dir = temp_output_dir("redacted-text");
        let now = unix_millis().saturating_add(100);
        let result = record(
            RecordRequest {
                output_dir: output_dir.clone(),
                session: Some("redacted".to_string()),
                text_mode: TextMode::Redacted,
                format: ImageFormat::Jpg,
                image_size_limit: ImageSizeLimit::FixedMaxEdge(800),
                fps: 30,
                settle_ms: 0,
                countdown_ms: 0,
            },
            Arc::new(FakeCaptureBackend::new()),
            Box::new(FakeEventSource::new(vec![
                RawInputEvent::KeyDown {
                    at_ms: now,
                    key: "s".to_string(),
                    text: Some("correct horse battery staple".to_string()),
                    modifiers: Default::default(),
                    repeat: false,
                },
                RawInputEvent::Control {
                    at_ms: now + 1,
                    control: ControlEvent::Stop,
                },
            ])),
        )
        .unwrap();
        assert!(!result.replayable);
        for path in [
            result.sequence_path,
            result.session_dir.join("actions/000001.md"),
            result.session_dir.join("events.md"),
        ] {
            let contents = fs::read_to_string(path).unwrap();
            assert!(!contents.contains("correct horse battery staple"));
        }
        let _ = fs::remove_dir_all(output_dir);
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ferrisgrid-record-test-{name}-{}-{}-{}",
            std::process::id(),
            unix_millis(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
