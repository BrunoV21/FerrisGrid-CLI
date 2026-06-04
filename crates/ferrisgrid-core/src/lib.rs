use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, FerrisError>;

#[derive(Debug, Clone)]
pub struct FerrisError {
    pub kind: ErrorKind,
    pub message: String,
}

impl FerrisError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl Display for FerrisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for FerrisError {}

impl From<std::io::Error> for FerrisError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Storage, error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Capture,
    Permission,
    Coordinate,
    Agent,
    Protocol,
    Execution,
    Storage,
    Platform,
    UserInterrupt,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture_error",
            Self::Permission => "permission_error",
            Self::Coordinate => "coordinate_error",
            Self::Agent => "agent_error",
            Self::Protocol => "protocol_error",
            Self::Execution => "execution_error",
            Self::Storage => "storage_error",
            Self::Platform => "platform_error",
            Self::UserInterrupt => "user_interrupt",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenInfo {
    pub screen_id: String,
    pub name: String,
    pub is_primary: bool,
    pub origin_x: i32,
    pub origin_y: i32,
    pub native_width: u32,
    pub native_height: u32,
    pub scale_factor: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedScreen {
    pub screen: ScreenInfo,
    pub image_width: u32,
    pub image_height: u32,
    pub screenshot_path: PathBuf,
    pub metadata_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateMode {
    Normalized1000,
    ImagePixels,
    NativePixels,
}

impl CoordinateMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normalized1000 => "normalized-1000",
            Self::ImagePixels => "image-pixels",
            Self::NativePixels => "native-pixels",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTarget {
    All,
    Screen(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFormat {
    Jpg,
    Png,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
        }
    }
}

impl std::str::FromStr for ImageFormat {
    type Err = FerrisError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "jpg" | "jpeg" => Ok(Self::Jpg),
            "png" => Ok(Self::Png),
            other => Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("unsupported image format: {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSizeLimit {
    Native,
    FixedMaxEdge(u32),
    Adaptive {
        min_long_edge: u32,
        min_short_edge: u32,
    },
}

impl ImageSizeLimit {
    pub fn description(self) -> String {
        match self {
            Self::Native => "native".to_string(),
            Self::FixedMaxEdge(edge) => edge.to_string(),
            Self::Adaptive {
                min_long_edge,
                min_short_edge,
            } => {
                format!("adaptive min_long_edge={min_long_edge} min_short_edge={min_short_edge}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObserveRequest {
    pub output_dir: PathBuf,
    pub session: Option<String>,
    pub screen_id: Option<String>,
    pub format: ImageFormat,
    pub grid_overlay: bool,
    pub image_size_limit: ImageSizeLimit,
}

#[derive(Debug, Clone)]
pub struct ObserveResult {
    pub session_dir: PathBuf,
    pub step: u32,
    pub coordinate_mode: CoordinateMode,
    pub image_size_limit: ImageSizeLimit,
    pub screens: Vec<CapturedScreen>,
}

#[derive(Debug, Clone)]
pub struct ActRequest {
    pub output_dir: PathBuf,
    pub session: Option<String>,
    pub default_screen_id: Option<String>,
    pub input_markdown: String,
    pub dry_run: bool,
    pub format: ImageFormat,
    pub grid_overlay: bool,
    pub image_size_limit: ImageSizeLimit,
}

#[derive(Debug, Clone)]
pub struct ActResult {
    pub session_dir: PathBuf,
    pub step: u32,
    pub action_summary: String,
    pub wait_after_ms: u64,
    pub result: String,
    pub dry_run: bool,
    pub image_size_limit: ImageSizeLimit,
    pub screens: Vec<CapturedScreen>,
}

#[derive(Debug, Clone)]
pub struct ActionErrorResult {
    pub session_dir: Option<PathBuf>,
    pub step: Option<u32>,
    pub error_type: String,
    pub reason: String,
    pub available_screens: Vec<CapturedScreen>,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub os: String,
    pub capture: String,
    pub input: String,
    pub output_dir: String,
    pub screens: Vec<ScreenInfo>,
    pub ffmpeg: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAction {
    pub status: ActionStatus,
    pub kind: Option<ActionKind>,
    pub wait_after_ms: Option<u64>,
    pub confidence: Option<f32>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStatus {
    Action,
    Done,
    Fail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Click {
        screen_id: Option<String>,
        x: i32,
        y: i32,
        button: MouseButton,
    },
    DoubleClick {
        screen_id: Option<String>,
        x: i32,
        y: i32,
        button: MouseButton,
    },
    RightClick {
        screen_id: Option<String>,
        x: i32,
        y: i32,
    },
    MoveMouse {
        screen_id: Option<String>,
        x: i32,
        y: i32,
    },
    Drag {
        screen_id: Option<String>,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        duration_ms: u64,
        button: MouseButton,
    },
    Scroll {
        screen_id: Option<String>,
        x: Option<i32>,
        y: Option<i32>,
        delta_x: i32,
        delta_y: i32,
    },
    Type {
        text: String,
    },
    PressKey {
        key: String,
    },
    Hotkey {
        keys: Vec<String>,
    },
    Wait {
        duration_ms: u64,
    },
}

impl ActionKind {
    pub fn screen_id(&self) -> Option<&str> {
        match self {
            Self::Click { screen_id, .. }
            | Self::DoubleClick { screen_id, .. }
            | Self::RightClick { screen_id, .. }
            | Self::MoveMouse { screen_id, .. }
            | Self::Drag { screen_id, .. }
            | Self::Scroll { screen_id, .. } => screen_id.as_deref(),
            Self::Type { .. } | Self::PressKey { .. } | Self::Hotkey { .. } | Self::Wait { .. } => {
                None
            }
        }
    }

    fn accepts_screen_id(&self) -> bool {
        matches!(
            self,
            Self::Click { .. }
                | Self::DoubleClick { .. }
                | Self::RightClick { .. }
                | Self::MoveMouse { .. }
                | Self::Drag { .. }
                | Self::Scroll { .. }
        )
    }

    fn requires_screen_id(&self) -> bool {
        match self {
            Self::Click { .. }
            | Self::DoubleClick { .. }
            | Self::RightClick { .. }
            | Self::MoveMouse { .. }
            | Self::Drag { .. } => true,
            Self::Scroll { x, y, .. } => x.is_some() || y.is_some(),
            Self::Type { .. } | Self::PressKey { .. } | Self::Hotkey { .. } | Self::Wait { .. } => {
                false
            }
        }
    }

    pub fn with_screen_id(self, resolved: Option<String>) -> Self {
        match self {
            Self::Click { x, y, button, .. } => Self::Click {
                screen_id: resolved,
                x,
                y,
                button,
            },
            Self::DoubleClick { x, y, button, .. } => Self::DoubleClick {
                screen_id: resolved,
                x,
                y,
                button,
            },
            Self::RightClick { x, y, .. } => Self::RightClick {
                screen_id: resolved,
                x,
                y,
            },
            Self::MoveMouse { x, y, .. } => Self::MoveMouse {
                screen_id: resolved,
                x,
                y,
            },
            Self::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
                duration_ms,
                button,
                ..
            } => Self::Drag {
                screen_id: resolved,
                from_x,
                from_y,
                to_x,
                to_y,
                duration_ms,
                button,
            },
            Self::Scroll {
                x,
                y,
                delta_x,
                delta_y,
                ..
            } => Self::Scroll {
                screen_id: resolved,
                x,
                y,
                delta_x,
                delta_y,
            },
            other => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NativeAction {
    Click {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    DoubleClick {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    RightClick {
        x: i32,
        y: i32,
    },
    MoveMouse {
        x: i32,
        y: i32,
    },
    Drag {
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        duration_ms: u64,
        button: MouseButton,
    },
    Scroll {
        x: Option<i32>,
        y: Option<i32>,
        delta_x: i32,
        delta_y: i32,
    },
    Type {
        text: String,
    },
    PressKey {
        key: String,
    },
    Hotkey {
        keys: Vec<String>,
    },
    Wait {
        duration_ms: u64,
    },
}

#[derive(Debug, Clone)]
pub struct InputExecution {
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct InputCapabilities {
    pub can_mouse: bool,
    pub can_keyboard: bool,
}

pub trait CaptureBackend {
    fn name(&self) -> &'static str;
    fn list_screens(&self) -> Result<Vec<ScreenInfo>>;
    fn capture(
        &self,
        target: CaptureTarget,
        frame_dir: &Path,
        format: &ImageFormat,
        grid_overlay: bool,
        image_size_limit: ImageSizeLimit,
    ) -> Result<Vec<CapturedScreen>>;
}

pub trait InputBackend {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> InputCapabilities;
    fn execute(&self, action: &NativeAction) -> Result<InputExecution>;
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("sessions"))?;
        let config = self.root.join("config.toml");
        if !config.exists() {
            fs::write(
                &config,
                "default_output_dir = \".ferrisgrid\"\nstorage_mode = \"all\"\n",
            )?;
        }
        Ok(())
    }

    pub fn resolve_session(
        &self,
        requested: Option<&str>,
        create_if_missing: bool,
    ) -> Result<PathBuf> {
        self.ensure_root()?;
        if let Some(value) = requested {
            let path = PathBuf::from(value);
            let session_dir = if path.exists() || value.contains('/') {
                path
            } else {
                self.root.join("sessions").join(value)
            };
            if session_dir.exists() || create_if_missing {
                self.ensure_session_dirs(&session_dir)?;
                return Ok(session_dir);
            }
            return Err(FerrisError::new(
                ErrorKind::Storage,
                format!("session not found: {}", session_dir.display()),
            ));
        }

        if let Some(latest) = self.latest_session()? {
            return Ok(latest);
        }

        if create_if_missing {
            return self.create_session();
        }

        Err(FerrisError::new(
            ErrorKind::Storage,
            "no existing session; run ferrisgrid observe first or pass --session",
        ))
    }

    pub fn create_session(&self) -> Result<PathBuf> {
        self.ensure_root()?;
        let session_id = new_session_id();
        let session_dir = self.root.join("sessions").join(session_id);
        self.ensure_session_dirs(&session_dir)?;
        Ok(session_dir)
    }

    pub fn latest_session(&self) -> Result<Option<PathBuf>> {
        let sessions_dir = self.root.join("sessions");
        if !sessions_dir.exists() {
            return Ok(None);
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(sessions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                entries.push(entry.path());
            }
        }
        entries.sort();
        Ok(entries.pop())
    }

    pub fn next_step(&self, session_dir: &Path) -> Result<u32> {
        let frames = session_dir.join("frames");
        fs::create_dir_all(&frames)?;
        let mut max_step = 0;
        for entry in fs::read_dir(frames)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(step) = name.parse::<u32>() {
                    max_step = max_step.max(step);
                }
            }
        }
        Ok(max_step + 1)
    }

    pub fn frame_dir(&self, session_dir: &Path, step: u32) -> Result<PathBuf> {
        let dir = session_dir.join("frames").join(format!("{step:06}"));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn write_manifest_if_missing(&self, session_dir: &Path) -> Result<()> {
        let manifest = session_dir.join("manifest.md");
        if !manifest.exists() {
            fs::write(
                manifest,
                format!(
                    "## FerrisGrid Session\n- session_id: {}\n- created_at_unix_ms: {}\n",
                    session_dir
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown"),
                    unix_millis()
                ),
            )?;
        }
        Ok(())
    }

    pub fn append_event(&self, session_dir: &Path, line: impl AsRef<str>) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(session_dir.join("events.md"))?;
        writeln!(file, "- {}", line.as_ref())?;
        Ok(())
    }

    pub fn write_action_files(
        &self,
        session_dir: &Path,
        step: u32,
        request: &str,
        parsed: &str,
        result: &str,
    ) -> Result<()> {
        let actions = session_dir.join("actions");
        fs::create_dir_all(&actions)?;
        fs::write(
            actions.join(format!("{step:06}.md")),
            format!(
                "## FerrisGrid Action\n- step: {step}\n\n### Request\n```text\n{}\n```\n\n### Parsed\n```text\n{}\n```\n\n### Result\n```text\n{}\n```\n",
                request.trim(),
                parsed.trim(),
                result.trim()
            ),
        )?;
        Ok(())
    }

    fn ensure_session_dirs(&self, session_dir: &Path) -> Result<()> {
        fs::create_dir_all(session_dir.join("frames"))?;
        self.write_manifest_if_missing(session_dir)?;
        Ok(())
    }
}

pub fn observe(request: ObserveRequest, capture: &dyn CaptureBackend) -> Result<ObserveResult> {
    let store = SessionStore::new(request.output_dir);
    let session_dir = store.resolve_session(request.session.as_deref(), true)?;
    let step = store.next_step(&session_dir)?;
    let frame_dir = store.frame_dir(&session_dir, step)?;
    let target = match request.screen_id {
        Some(id) => CaptureTarget::Screen(resolve_primary_alias(&id, &capture.list_screens()?)),
        None => CaptureTarget::All,
    };
    let screens = match capture.capture(
        target,
        &frame_dir,
        &request.format,
        request.grid_overlay,
        request.image_size_limit,
    ) {
        Ok(screens) => screens,
        Err(error) => {
            remove_empty_dir(&frame_dir);
            return Err(error);
        }
    };
    store.append_event(
        &session_dir,
        format!(
            "{} frame_captured step={} screens={}",
            unix_millis(),
            step,
            screens.len()
        ),
    )?;
    Ok(ObserveResult {
        session_dir,
        step,
        coordinate_mode: CoordinateMode::Normalized1000,
        image_size_limit: request.image_size_limit,
        screens,
    })
}

pub fn act(
    request: ActRequest,
    capture: &dyn CaptureBackend,
    input: &dyn InputBackend,
) -> std::result::Result<ActResult, ActionErrorResult> {
    match act_inner(request, capture, input) {
        Ok(result) => Ok(result),
        Err((error, context)) => Err(renderable_error(error, context)),
    }
}

fn act_inner(
    request: ActRequest,
    capture: &dyn CaptureBackend,
    input: &dyn InputBackend,
) -> std::result::Result<ActResult, (FerrisError, ErrorContext)> {
    let store = SessionStore::new(request.output_dir);
    let session_dir = store
        .resolve_session(request.session.as_deref(), false)
        .map_err(|error| (error, ErrorContext::default()))?;
    let step = store
        .next_step(&session_dir)
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
    let action = parse_action_block(&request.input_markdown)
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;

    if action.status == ActionStatus::Done || action.status == ActionStatus::Fail {
        let result = if action.status == ActionStatus::Done {
            "done"
        } else {
            "fail"
        };
        store
            .write_action_files(
                &session_dir,
                step,
                &request.input_markdown,
                &format!("{action:?}"),
                result,
            )
            .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
        return Ok(ActResult {
            session_dir,
            step,
            action_summary: result.to_string(),
            wait_after_ms: 0,
            result: result.to_string(),
            dry_run: request.dry_run,
            image_size_limit: request.image_size_limit,
            screens: Vec::new(),
        });
    }

    let kind = action.kind.clone().ok_or_else(|| {
        (
            FerrisError::new(
                ErrorKind::Protocol,
                "status action requires an action field",
            ),
            ErrorContext::with_session(session_dir.clone()),
        )
    })?;

    validate_policy(&kind)
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
    let screens = capture
        .list_screens()
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
    let requested_screen_id = kind
        .screen_id()
        .or(request.default_screen_id.as_deref())
        .filter(|_| kind.accepts_screen_id());
    let resolved_screen = if kind.requires_screen_id() || requested_screen_id.is_some() {
        resolve_action_screen(requested_screen_id, &screens).map_err(|error| {
            let mut context = ErrorContext::with_session(session_dir.clone());
            context.available_screens = capture_latest_screens(
                &store,
                &session_dir,
                step,
                capture,
                &request.format,
                request.image_size_limit,
            )
            .unwrap_or_default();
            (error, context)
        })?
    } else {
        None
    };

    let resolved_kind = kind.with_screen_id(
        resolved_screen
            .as_ref()
            .map(|screen| screen.screen_id.clone()),
    );
    let native = to_native_action(&resolved_kind, resolved_screen)
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;

    let execution = if request.dry_run {
        InputExecution {
            summary: "dry_run".to_string(),
        }
    } else {
        input
            .execute(&native)
            .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?
    };

    let wait_after_ms = action.wait_after_ms.unwrap_or(0);
    if wait_after_ms > 0 && !request.dry_run {
        thread::sleep(Duration::from_millis(wait_after_ms));
    }

    let frame_dir = store
        .frame_dir(&session_dir, step)
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
    let target = match resolved_screen {
        Some(screen) => CaptureTarget::Screen(screen.screen_id.clone()),
        None => CaptureTarget::All,
    };
    let captured = match capture.capture(
        target,
        &frame_dir,
        &request.format,
        request.grid_overlay,
        request.image_size_limit,
    ) {
        Ok(captured) => captured,
        Err(error) => {
            remove_empty_dir(&frame_dir);
            return Err((error, ErrorContext::with_session(session_dir.clone())));
        }
    };
    let summary = action_summary(&resolved_kind);
    let parsed_summary = action_summary_with_wait_after(&resolved_kind, wait_after_ms);
    let result_text = if request.dry_run {
        "dry_run"
    } else {
        "success"
    };
    store
        .write_action_files(
            &session_dir,
            step,
            &request.input_markdown,
            &parsed_summary,
            &execution.summary,
        )
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;
    store
        .append_event(
            &session_dir,
            format!(
                "{} action_executed step={} action={} wait_after_ms={} result={}",
                unix_millis(),
                step,
                summary,
                wait_after_ms,
                result_text
            ),
        )
        .map_err(|error| (error, ErrorContext::with_session(session_dir.clone())))?;

    Ok(ActResult {
        session_dir,
        step,
        action_summary: summary,
        wait_after_ms,
        result: result_text.to_string(),
        dry_run: request.dry_run,
        image_size_limit: request.image_size_limit,
        screens: captured,
    })
}

#[derive(Default)]
struct ErrorContext {
    session_dir: Option<PathBuf>,
    step: Option<u32>,
    available_screens: Vec<CapturedScreen>,
}

impl ErrorContext {
    fn with_session(session_dir: PathBuf) -> Self {
        Self {
            session_dir: Some(session_dir),
            step: None,
            available_screens: Vec::new(),
        }
    }
}

fn renderable_error(error: FerrisError, context: ErrorContext) -> ActionErrorResult {
    ActionErrorResult {
        session_dir: context.session_dir,
        step: context.step,
        error_type: error.kind.as_str().to_string(),
        reason: error.message,
        available_screens: context.available_screens,
    }
}

fn capture_latest_screens(
    store: &SessionStore,
    session_dir: &Path,
    step: u32,
    capture: &dyn CaptureBackend,
    format: &ImageFormat,
    image_size_limit: ImageSizeLimit,
) -> Result<Vec<CapturedScreen>> {
    let frame_dir = store.frame_dir(session_dir, step)?;
    capture.capture(
        CaptureTarget::All,
        &frame_dir,
        format,
        true,
        image_size_limit,
    )
}

fn remove_empty_dir(path: &Path) {
    let _ = fs::remove_dir(path);
}

pub fn render_observation(result: &ObserveResult) -> String {
    let mut out = String::new();
    out.push_str("## FerrisGrid Observation\n");
    out.push_str(&format!("- session: {}\n", result.session_dir.display()));
    out.push_str(&format!("- step: {}\n", result.step));
    out.push_str(&format!(
        "- coordinate_mode: {}\n",
        result.coordinate_mode.as_str()
    ));
    out.push_str(&format!(
        "- image_size_limit: {}\n",
        result.image_size_limit.description()
    ));
    out.push_str("- coordinate_range: x=0..1000 y=0..1000 origin=top_left scope=screen_local\n");
    out.push_str(
        "- action_coordinates: use these x/y values with ferrisgrid act; include screen_id when more than one screen is listed\n",
    );
    out.push_str(&format!("- screens: {}\n", result.screens.len()));
    for screen in &result.screens {
        out.push_str(&format!(
            "- screen: {} primary={} image={}x{} native={}x{} origin={},{} coords=x:0..1000,y:0..1000 screenshot={} metadata={}\n",
            screen.screen.screen_id,
            screen.screen.is_primary,
            screen.image_width,
            screen.image_height,
            screen.screen.native_width,
            screen.screen.native_height,
            screen.screen.origin_x,
            screen.screen.origin_y,
            screen.screenshot_path.display(),
            screen.metadata_path.display()
        ));
        out.push_str(&format!(
            "- map: {} image_x=round(x/1000*{}) image_y=round(y/1000*{}) native_x={}+round(x/1000*{}) native_y={}+round(y/1000*{})\n",
            screen.screen.screen_id,
            screen.image_width.saturating_sub(1),
            screen.image_height.saturating_sub(1),
            screen.screen.origin_x,
            screen.screen.native_width,
            screen.screen.origin_y,
            screen.screen.native_height
        ));
    }
    out
}

pub fn render_action_result(result: &ActResult) -> String {
    let mut out = String::new();
    out.push_str("## FerrisGrid Action Result\n");
    out.push_str(&format!("- session: {}\n", result.session_dir.display()));
    out.push_str(&format!("- step: {}\n", result.step));
    out.push_str(&format!("- action: {}\n", result.action_summary));
    if result.wait_after_ms > 0 {
        out.push_str(&format!("- wait_after_ms: {}\n", result.wait_after_ms));
    }
    out.push_str(&format!("- result: {}\n", result.result));
    out.push_str(&format!("- screens: {}\n", result.screens.len()));
    if result.screens.is_empty() {
        out.push_str("- note: no post-action screenshot captured for terminal status\n");
        return out;
    }
    out.push_str("- coordinate_mode: normalized-1000\n");
    out.push_str(&format!(
        "- image_size_limit: {}\n",
        result.image_size_limit.description()
    ));
    out.push_str("- coordinate_range: x=0..1000 y=0..1000 origin=top_left scope=screen_local\n");
    for screen in &result.screens {
        out.push_str(&format!(
            "- screen: {} primary={} image={}x{} native={}x{} origin={},{} coords=x:0..1000,y:0..1000 screenshot={} metadata={}\n",
            screen.screen.screen_id,
            screen.screen.is_primary,
            screen.image_width,
            screen.image_height,
            screen.screen.native_width,
            screen.screen.native_height,
            screen.screen.origin_x,
            screen.screen.origin_y,
            screen.screenshot_path.display(),
            screen.metadata_path.display()
        ));
        out.push_str(&format!(
            "- map: {} image_x=round(x/1000*{}) image_y=round(y/1000*{}) native_x={}+round(x/1000*{}) native_y={}+round(y/1000*{})\n",
            screen.screen.screen_id,
            screen.image_width.saturating_sub(1),
            screen.image_height.saturating_sub(1),
            screen.screen.origin_x,
            screen.screen.native_width,
            screen.screen.origin_y,
            screen.screen.native_height
        ));
    }
    out
}

pub fn render_action_error(error: &ActionErrorResult) -> String {
    let mut out = String::new();
    out.push_str("## FerrisGrid Action Error\n");
    out.push_str(&format!("- type: {}\n", error.error_type));
    out.push_str("- result: rejected\n");
    out.push_str(&format!("- reason: {}\n", error.reason));
    if let Some(session) = &error.session_dir {
        out.push_str(&format!("- session: {}\n", session.display()));
    }
    for screen in &error.available_screens {
        out.push_str(&format!(
            "- available_screen: {} coords=x:0..1000,y:0..1000 screenshot={} metadata={}\n",
            screen.screen.screen_id,
            screen.screenshot_path.display(),
            screen.metadata_path.display()
        ));
    }
    out
}

pub fn render_doctor(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("## FerrisGrid Doctor\n");
    out.push_str(&format!("- os: {}\n", report.os));
    out.push_str(&format!("- capture: {}\n", report.capture));
    out.push_str(&format!("- input: {}\n", report.input));
    out.push_str(&format!("- output_directory: {}\n", report.output_dir));
    out.push_str(&format!("- screens: {}\n", report.screens.len()));
    for screen in &report.screens {
        out.push_str(&format!(
            "- screen: {} primary={} origin={},{} native={}x{} scale={}\n",
            screen.screen_id,
            screen.is_primary,
            screen.origin_x,
            screen.origin_y,
            screen.native_width,
            screen.native_height,
            screen.scale_factor
        ));
    }
    out.push_str(&format!("- ffmpeg: {}\n", report.ffmpeg));
    out
}

pub fn parse_action_block(markdown: &str) -> Result<AgentAction> {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return Err(FerrisError::new(ErrorKind::Protocol, "empty action input"));
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "JSON action input is not supported; use compact Markdown",
        ));
    }

    let mut fields = BTreeMap::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("invalid action line, expected key: value: {line}"),
            ));
        };
        fields.insert(key.trim().to_string(), value.trim().to_string());
    }
    if fields.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "action input must contain compact Markdown key/value lines",
        ));
    }

    let status = match fields.get("status").map(String::as_str).unwrap_or("action") {
        "action" => ActionStatus::Action,
        "done" => ActionStatus::Done,
        "fail" => ActionStatus::Fail,
        other => {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("unsupported status: {other}"),
            ));
        }
    };
    let confidence = match fields.get("confidence") {
        Some(value) => Some(parse_f32(value, "confidence")?),
        None => None,
    };
    let reason = fields.get("reason").cloned();
    let wait_after_ms = match fields.get("wait_after_ms") {
        Some(value) => Some(parse_u64(value, "wait_after_ms")?),
        None => None,
    };
    validate_wait_after(wait_after_ms)?;
    let kind = if status == ActionStatus::Action {
        Some(parse_action_kind(&fields)?)
    } else {
        None
    };

    Ok(AgentAction {
        status,
        kind,
        wait_after_ms,
        confidence,
        reason,
    })
}

fn validate_wait_after(wait_after_ms: Option<u64>) -> Result<()> {
    if let Some(wait_after_ms) = wait_after_ms {
        if wait_after_ms > 30_000 {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                "wait_after_ms exceeds 30000 ms",
            ));
        }
    }
    Ok(())
}

fn parse_action_kind(fields: &BTreeMap<String, String>) -> Result<ActionKind> {
    let action = required(fields, "action")?;
    let screen_id = fields.get("screen_id").cloned();
    match action.as_str() {
        "click" => Ok(ActionKind::Click {
            screen_id,
            x: parse_i32_required(fields, "x")?,
            y: parse_i32_required(fields, "y")?,
            button: parse_button(fields.get("button").map(String::as_str).unwrap_or("left"))?,
        }),
        "double_click" => Ok(ActionKind::DoubleClick {
            screen_id,
            x: parse_i32_required(fields, "x")?,
            y: parse_i32_required(fields, "y")?,
            button: parse_button(fields.get("button").map(String::as_str).unwrap_or("left"))?,
        }),
        "right_click" => Ok(ActionKind::RightClick {
            screen_id,
            x: parse_i32_required(fields, "x")?,
            y: parse_i32_required(fields, "y")?,
        }),
        "move_mouse" => Ok(ActionKind::MoveMouse {
            screen_id,
            x: parse_i32_required(fields, "x")?,
            y: parse_i32_required(fields, "y")?,
        }),
        "drag" => Ok(ActionKind::Drag {
            screen_id,
            from_x: parse_i32_required(fields, "from_x")?,
            from_y: parse_i32_required(fields, "from_y")?,
            to_x: parse_i32_required(fields, "to_x")?,
            to_y: parse_i32_required(fields, "to_y")?,
            duration_ms: parse_u64(
                fields
                    .get("duration_ms")
                    .map(String::as_str)
                    .unwrap_or("450"),
                "duration_ms",
            )?,
            button: parse_button(fields.get("button").map(String::as_str).unwrap_or("left"))?,
        }),
        "scroll" => Ok(ActionKind::Scroll {
            screen_id,
            x: parse_i32_optional(fields.get("x").map(String::as_str), "x")?,
            y: parse_i32_optional(fields.get("y").map(String::as_str), "y")?,
            delta_x: parse_i32_optional(fields.get("delta_x").map(String::as_str), "delta_x")?
                .unwrap_or(0),
            delta_y: parse_i32_required(fields, "delta_y")?,
        }),
        "type" => Ok(ActionKind::Type {
            text: required(fields, "text")?,
        }),
        "press_key" => Ok(ActionKind::PressKey {
            key: required(fields, "key")?,
        }),
        "hotkey" => Ok(ActionKind::Hotkey {
            keys: required(fields, "keys")?
                .split('+')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        }),
        "wait" => Ok(ActionKind::Wait {
            duration_ms: parse_u64(&required(fields, "duration_ms")?, "duration_ms")?,
        }),
        other => Err(FerrisError::new(
            ErrorKind::Protocol,
            format!("unknown action: {other}"),
        )),
    }
}

fn validate_policy(action: &ActionKind) -> Result<()> {
    match action {
        ActionKind::Click { x, y, .. }
        | ActionKind::DoubleClick { x, y, .. }
        | ActionKind::RightClick { x, y, .. }
        | ActionKind::MoveMouse { x, y, .. } => validate_agent_point(*x, *y),
        ActionKind::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            ..
        } => {
            validate_agent_point(*from_x, *from_y)?;
            validate_agent_point(*to_x, *to_y)?;
            if *duration_ms > 5_000 {
                return Err(FerrisError::new(
                    ErrorKind::Protocol,
                    "drag duration exceeds 5000 ms",
                ));
            }
            Ok(())
        }
        ActionKind::Scroll {
            x,
            y,
            delta_x,
            delta_y,
            ..
        } => {
            match (x, y) {
                (Some(x), Some(y)) => validate_agent_point(*x, *y)?,
                (None, None) => {}
                _ => {
                    return Err(FerrisError::new(
                        ErrorKind::Protocol,
                        "scroll x and y must be supplied together",
                    ));
                }
            }
            if delta_x.abs() > 2_000 || delta_y.abs() > 2_000 {
                return Err(FerrisError::new(
                    ErrorKind::Protocol,
                    "scroll delta exceeds 2000",
                ));
            }
            Ok(())
        }
        ActionKind::Type { text } => {
            if text.chars().count() > 500 {
                return Err(FerrisError::new(
                    ErrorKind::Protocol,
                    "typed text exceeds 500 characters",
                ));
            }
            Ok(())
        }
        ActionKind::Hotkey { keys } => {
            if keys.is_empty() || keys.len() > 4 {
                return Err(FerrisError::new(
                    ErrorKind::Protocol,
                    "hotkey must contain 1 to 4 keys",
                ));
            }
            Ok(())
        }
        ActionKind::PressKey { key } => {
            if key.trim().is_empty() {
                return Err(FerrisError::new(ErrorKind::Protocol, "key is required"));
            }
            Ok(())
        }
        ActionKind::Wait { duration_ms } => {
            if *duration_ms > 30_000 {
                return Err(FerrisError::new(
                    ErrorKind::Protocol,
                    "wait duration exceeds 30000 ms",
                ));
            }
            Ok(())
        }
    }
}

fn validate_agent_point(x: i32, y: i32) -> Result<()> {
    if !(0..=1000).contains(&x) || !(0..=1000).contains(&y) {
        return Err(FerrisError::new(
            ErrorKind::Coordinate,
            "coordinates must be within 0..1000",
        ));
    }
    Ok(())
}

fn resolve_action_screen<'a>(
    screen_id: Option<&str>,
    screens: &'a [ScreenInfo],
) -> Result<Option<&'a ScreenInfo>> {
    if screens.is_empty() {
        return Err(FerrisError::new(ErrorKind::Capture, "no screens available"));
    }
    if let Some(id) = screen_id {
        let id = resolve_primary_alias(id, screens);
        return screens
            .iter()
            .find(|screen| screen.screen_id == id)
            .map(Some)
            .ok_or_else(|| {
                FerrisError::new(ErrorKind::Coordinate, format!("unknown screen_id: {id}"))
            });
    }
    if screens.len() == 1 {
        return Ok(screens.first());
    }
    Err(FerrisError::new(
        ErrorKind::Coordinate,
        "screen_id is required because multiple screens are active",
    ))
}

fn resolve_primary_alias(id: &str, screens: &[ScreenInfo]) -> String {
    if id == "primary" {
        if let Some(primary) = screens.iter().find(|screen| screen.is_primary) {
            return primary.screen_id.clone();
        }
    }
    id.to_string()
}

fn to_native_action(action: &ActionKind, screen: Option<&ScreenInfo>) -> Result<NativeAction> {
    match action {
        ActionKind::Click { x, y, button, .. } => {
            let (x, y) = map_point(
                *x,
                *y,
                screen.ok_or_else(|| {
                    FerrisError::new(ErrorKind::Coordinate, "screen_id required for click")
                })?,
            )?;
            Ok(NativeAction::Click {
                x,
                y,
                button: *button,
            })
        }
        ActionKind::DoubleClick { x, y, button, .. } => {
            let (x, y) = map_point(
                *x,
                *y,
                screen.ok_or_else(|| {
                    FerrisError::new(ErrorKind::Coordinate, "screen_id required for double_click")
                })?,
            )?;
            Ok(NativeAction::DoubleClick {
                x,
                y,
                button: *button,
            })
        }
        ActionKind::RightClick { x, y, .. } => {
            let (x, y) = map_point(
                *x,
                *y,
                screen.ok_or_else(|| {
                    FerrisError::new(ErrorKind::Coordinate, "screen_id required for right_click")
                })?,
            )?;
            Ok(NativeAction::RightClick { x, y })
        }
        ActionKind::MoveMouse { x, y, .. } => {
            let (x, y) = map_point(
                *x,
                *y,
                screen.ok_or_else(|| {
                    FerrisError::new(ErrorKind::Coordinate, "screen_id required for move_mouse")
                })?,
            )?;
            Ok(NativeAction::MoveMouse { x, y })
        }
        ActionKind::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button,
            ..
        } => {
            let screen = screen.ok_or_else(|| {
                FerrisError::new(ErrorKind::Coordinate, "screen_id required for drag")
            })?;
            let (from_x, from_y) = map_point(*from_x, *from_y, screen)?;
            let (to_x, to_y) = map_point(*to_x, *to_y, screen)?;
            Ok(NativeAction::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
                duration_ms: *duration_ms,
                button: *button,
            })
        }
        ActionKind::Scroll {
            x,
            y,
            delta_x,
            delta_y,
            ..
        } => {
            let point = match (x, y, screen) {
                (Some(x), Some(y), Some(screen)) => Some(map_point(*x, *y, screen)?),
                _ => None,
            };
            Ok(NativeAction::Scroll {
                x: point.map(|value| value.0),
                y: point.map(|value| value.1),
                delta_x: *delta_x,
                delta_y: *delta_y,
            })
        }
        ActionKind::Type { text } => Ok(NativeAction::Type { text: text.clone() }),
        ActionKind::PressKey { key } => Ok(NativeAction::PressKey { key: key.clone() }),
        ActionKind::Hotkey { keys } => Ok(NativeAction::Hotkey { keys: keys.clone() }),
        ActionKind::Wait { duration_ms } => Ok(NativeAction::Wait {
            duration_ms: *duration_ms,
        }),
    }
}

pub fn map_point(agent_x: i32, agent_y: i32, screen: &ScreenInfo) -> Result<(i32, i32)> {
    validate_agent_point(agent_x, agent_y)?;
    let native_x =
        screen.origin_x + ((agent_x as f64 / 1000.0) * screen.native_width as f64).round() as i32;
    let native_y =
        screen.origin_y + ((agent_y as f64 / 1000.0) * screen.native_height as f64).round() as i32;
    let max_x = screen.origin_x + screen.native_width.saturating_sub(1) as i32;
    let max_y = screen.origin_y + screen.native_height.saturating_sub(1) as i32;
    Ok((
        native_x.clamp(screen.origin_x, max_x),
        native_y.clamp(screen.origin_y, max_y),
    ))
}

fn action_summary(action: &ActionKind) -> String {
    match action {
        ActionKind::Click {
            screen_id,
            x,
            y,
            button,
        } => format!(
            "click screen_id={} x={} y={} button={}",
            screen_id.as_deref().unwrap_or(""),
            x,
            y,
            button.as_str()
        ),
        ActionKind::DoubleClick {
            screen_id,
            x,
            y,
            button,
        } => format!(
            "double_click screen_id={} x={} y={} button={}",
            screen_id.as_deref().unwrap_or(""),
            x,
            y,
            button.as_str()
        ),
        ActionKind::RightClick { screen_id, x, y } => format!(
            "right_click screen_id={} x={} y={}",
            screen_id.as_deref().unwrap_or(""),
            x,
            y
        ),
        ActionKind::MoveMouse { screen_id, x, y } => format!(
            "move_mouse screen_id={} x={} y={}",
            screen_id.as_deref().unwrap_or(""),
            x,
            y
        ),
        ActionKind::Drag {
            screen_id,
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button,
        } => format!(
            "drag screen_id={} from_x={} from_y={} to_x={} to_y={} duration_ms={} button={}",
            screen_id.as_deref().unwrap_or(""),
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button.as_str()
        ),
        ActionKind::Scroll {
            screen_id,
            x,
            y,
            delta_x,
            delta_y,
        } => format!(
            "scroll screen_id={} x={} y={} delta_x={} delta_y={}",
            screen_id.as_deref().unwrap_or(""),
            x.map(|v| v.to_string()).unwrap_or_default(),
            y.map(|v| v.to_string()).unwrap_or_default(),
            delta_x,
            delta_y
        ),
        ActionKind::Type { .. } => "type text=<redacted>".to_string(),
        ActionKind::PressKey { key } => format!("press_key key={key}"),
        ActionKind::Hotkey { keys } => format!("hotkey keys={}", keys.join("+")),
        ActionKind::Wait { duration_ms } => format!("wait duration_ms={duration_ms}"),
    }
}

fn action_summary_with_wait_after(action: &ActionKind, wait_after_ms: u64) -> String {
    let summary = action_summary(action);
    if wait_after_ms == 0 {
        summary
    } else {
        format!("{summary}\nwait_after_ms={wait_after_ms}")
    }
}

fn required(fields: &BTreeMap<String, String>, key: &str) -> Result<String> {
    fields
        .get(key)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| FerrisError::new(ErrorKind::Protocol, format!("{key} is required")))
}

fn parse_i32_required(fields: &BTreeMap<String, String>, key: &str) -> Result<i32> {
    parse_i32(&required(fields, key)?, key)
}

fn parse_i32_optional(value: Option<&str>, key: &str) -> Result<Option<i32>> {
    value.map(|value| parse_i32(value, key)).transpose()
}

fn parse_i32(value: &str, key: &str) -> Result<i32> {
    value
        .parse::<i32>()
        .map_err(|_| FerrisError::new(ErrorKind::Protocol, format!("{key} must be an integer")))
}

fn parse_u64(value: &str, key: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| FerrisError::new(ErrorKind::Protocol, format!("{key} must be an integer")))
}

fn parse_f32(value: &str, key: &str) -> Result<f32> {
    value
        .parse::<f32>()
        .map_err(|_| FerrisError::new(ErrorKind::Protocol, format!("{key} must be a number")))
}

fn parse_button(value: &str) -> Result<MouseButton> {
    match value {
        "left" => Ok(MouseButton::Left),
        "right" => Ok(MouseButton::Right),
        "middle" => Ok(MouseButton::Middle),
        other => Err(FerrisError::new(
            ErrorKind::Protocol,
            format!("unsupported mouse button: {other}"),
        )),
    }
}

fn new_session_id() -> String {
    format!("{}-{}", unix_millis(), std::process::id())
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestCaptureBackend;

    impl CaptureBackend for TestCaptureBackend {
        fn name(&self) -> &'static str {
            "test"
        }

        fn list_screens(&self) -> Result<Vec<ScreenInfo>> {
            Ok(vec![
                screen(),
                ScreenInfo {
                    screen_id: "screen-2".to_string(),
                    name: "Test 2".to_string(),
                    is_primary: false,
                    origin_x: 3024,
                    origin_y: 0,
                    native_width: 2560,
                    native_height: 1440,
                    scale_factor: 1.0,
                },
            ])
        }

        fn capture(
            &self,
            target: CaptureTarget,
            frame_dir: &Path,
            format: &ImageFormat,
            _grid_overlay: bool,
            _image_size_limit: ImageSizeLimit,
        ) -> Result<Vec<CapturedScreen>> {
            let screens = self.list_screens()?;
            let selected: Vec<ScreenInfo> = match target {
                CaptureTarget::All => screens,
                CaptureTarget::Screen(id) => screens
                    .into_iter()
                    .filter(|screen| screen.screen_id == id)
                    .collect(),
            };
            Ok(selected
                .into_iter()
                .map(|screen| CapturedScreen {
                    screenshot_path: frame_dir.join(format!(
                        "{}.{}",
                        screen.screen_id,
                        format.extension()
                    )),
                    metadata_path: frame_dir.join(format!("{}.meta.md", screen.screen_id)),
                    image_width: 800,
                    image_height: 520,
                    screen,
                })
                .collect())
        }
    }

    struct TestInputBackend;

    impl InputBackend for TestInputBackend {
        fn name(&self) -> &'static str {
            "test"
        }

        fn capabilities(&self) -> InputCapabilities {
            InputCapabilities {
                can_mouse: true,
                can_keyboard: true,
            }
        }

        fn execute(&self, action: &NativeAction) -> Result<InputExecution> {
            Ok(InputExecution {
                summary: format!("{action:?}"),
            })
        }
    }

    fn screen() -> ScreenInfo {
        ScreenInfo {
            screen_id: "screen-1".to_string(),
            name: "Test".to_string(),
            is_primary: true,
            origin_x: 0,
            origin_y: 0,
            native_width: 3024,
            native_height: 1964,
            scale_factor: 2.0,
        }
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "ferrisgrid-core-test-{name}-{}-{nonce}-{counter}",
            std::process::id()
        ))
    }

    fn create_test_session(output_dir: &Path) {
        let store = SessionStore::new(output_dir);
        let session = store.create_session().unwrap();
        let frame_dir = store.frame_dir(&session, 1).unwrap();
        fs::write(frame_dir.join("screen-1.jpg"), "test").unwrap();
    }

    fn test_act_request(output_dir: PathBuf, input_markdown: &str) -> ActRequest {
        ActRequest {
            output_dir,
            session: None,
            default_screen_id: None,
            input_markdown: input_markdown.to_string(),
            dry_run: true,
            format: ImageFormat::Jpg,
            grid_overlay: false,
            image_size_limit: ImageSizeLimit::FixedMaxEdge(800),
        }
    }

    #[test]
    fn maps_normalized_center_to_native_center() {
        assert_eq!(map_point(500, 500, &screen()).unwrap(), (1512, 982));
    }

    #[test]
    fn rejects_out_of_bounds_coordinates() {
        assert!(map_point(1001, 500, &screen()).is_err());
    }

    #[test]
    fn parses_click_action_block() {
        let action = parse_action_block(
            "status: action\naction: click\nscreen_id: screen-1\nx: 742\ny: 611\nbutton: left\n",
        )
        .unwrap();
        assert_eq!(action.status, ActionStatus::Action);
        assert!(matches!(action.kind, Some(ActionKind::Click { .. })));
        assert_eq!(action.wait_after_ms, None);
    }

    #[test]
    fn parses_wait_after_ms() {
        let action = parse_action_block(
            "status: action\naction: click\nscreen_id: screen-1\nx: 742\ny: 611\nbutton: left\nwait_after_ms: 750\n",
        )
        .unwrap();
        assert_eq!(action.wait_after_ms, Some(750));
    }

    #[test]
    fn rejects_excessive_wait_after_ms() {
        let error = parse_action_block(
            "status: action\naction: click\nscreen_id: screen-1\nx: 742\ny: 611\nwait_after_ms: 30001\n",
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Protocol);
        assert!(error.message.contains("wait_after_ms"));
    }

    #[test]
    fn rejects_json_action_input() {
        assert!(parse_action_block("{\"action\":\"click\"}").is_err());
    }

    #[test]
    fn rejects_missing_screen_in_multi_screen_context() {
        let screens = vec![
            screen(),
            ScreenInfo {
                screen_id: "screen-2".to_string(),
                is_primary: false,
                ..screen()
            },
        ];
        assert!(resolve_action_screen(None, &screens).is_err());
    }

    #[test]
    fn multi_screen_wait_does_not_require_screen_id() {
        let output_dir = temp_output_dir("wait-no-screen");
        create_test_session(&output_dir);
        let result = act(
            test_act_request(output_dir.clone(), "action: wait\nduration_ms: 1\n"),
            &TestCaptureBackend,
            &TestInputBackend,
        )
        .unwrap();

        assert_eq!(result.action_summary, "wait duration_ms=1");
        assert_eq!(result.screens.len(), 2);
        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn default_screen_id_disambiguates_pointer_action() {
        let output_dir = temp_output_dir("default-screen");
        create_test_session(&output_dir);
        let mut request = test_act_request(output_dir.clone(), "action: click\nx: 500\ny: 500\n");
        request.default_screen_id = Some("screen-1".to_string());

        let result = act(request, &TestCaptureBackend, &TestInputBackend).unwrap();

        assert!(result.action_summary.contains("click screen_id=screen-1"));
        assert_eq!(result.screens.len(), 1);
        assert_eq!(result.screens[0].screen.screen_id, "screen-1");
        let _ = fs::remove_dir_all(output_dir);
    }

    #[test]
    fn rejects_partial_scroll_point() {
        let action = parse_action_block("action: scroll\nx: 500\ndelta_y: -120\n").unwrap();

        let error = validate_policy(&action.kind.unwrap()).unwrap_err();

        assert_eq!(error.kind, ErrorKind::Protocol);
        assert!(error.message.contains("x and y"));
    }

    #[test]
    fn terminal_action_result_reports_no_screens() {
        let rendered = render_action_result(&ActResult {
            session_dir: PathBuf::from(".ferrisgrid/sessions/test"),
            step: 2,
            action_summary: "done".to_string(),
            wait_after_ms: 0,
            result: "done".to_string(),
            dry_run: false,
            image_size_limit: ImageSizeLimit::FixedMaxEdge(800),
            screens: Vec::new(),
        });

        assert!(rendered.contains("- screens: 0"));
        assert!(rendered.contains("no post-action screenshot"));
        assert!(!rendered.contains("- coordinate_mode:"));
    }
}
