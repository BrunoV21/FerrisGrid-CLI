use ferrisgrid_capture::backend_by_name as capture_backend;
use ferrisgrid_core::{
    ActRequest, DoctorReport, ImageFormat, ImageSizeLimit, ObserveRequest, SessionStore, act,
    observe, render_action_error, render_action_result, render_doctor, render_observation,
};
use ferrisgrid_export::{RecapOptions, VideoFormat, recap_with_options, render_recap};
use ferrisgrid_input::backend_by_name as input_backend;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

const FAST_IMAGE_EDGE: u32 = 800;
const BALANCED_MIN_LONG_EDGE: u32 = 800;
const BALANCED_MIN_SHORT_EDGE: u32 = 500;
const DETAIL_IMAGE_EDGE: u32 = 1920;

macro_rules! docs_url {
    () => {
        "https://brunov21.github.io/FerrisGrid-CLI/"
    };
}

fn main() {
    if let Err(error) = run() {
        eprintln!(
            "## FerrisGrid Error\n- type: {}\n- reason: {}\n",
            error.kind.as_str(),
            error.message
        );
        process::exit(1);
    }
}

fn run() -> ferrisgrid_core::Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help(HelpTopic::Root);
        return Ok(());
    }
    let command = args.remove(0);
    match command.as_str() {
        "observe" if is_help_request(&args) => {
            print_help(HelpTopic::Observe);
            Ok(())
        }
        "observe" => command_observe(args),
        "act" if is_help_request(&args) => {
            print_help(HelpTopic::Act);
            Ok(())
        }
        "act" => command_act(args),
        "doctor" if is_help_request(&args) => {
            print_help(HelpTopic::Doctor);
            Ok(())
        }
        "doctor" => command_doctor(args),
        "recap" if is_help_request(&args) => {
            print_help(HelpTopic::Recap);
            Ok(())
        }
        "recap" => command_recap(args),
        "clear" if is_help_request(&args) => {
            print_help(HelpTopic::Clear);
            Ok(())
        }
        "clear" => command_clear(args),
        "-h" | "--help" | "help" => {
            let topic = match args.first() {
                Some(value) => parse_help_topic(value)?,
                None => HelpTopic::Root,
            };
            print_help(topic);
            Ok(())
        }
        other => Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("unknown command: {other}"),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpTopic {
    Root,
    Observe,
    Act,
    Doctor,
    Recap,
    Clear,
}

fn is_help_request(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
}

fn parse_help_topic(value: &str) -> ferrisgrid_core::Result<HelpTopic> {
    match value {
        "observe" => Ok(HelpTopic::Observe),
        "act" => Ok(HelpTopic::Act),
        "doctor" => Ok(HelpTopic::Doctor),
        "recap" => Ok(HelpTopic::Recap),
        "clear" => Ok(HelpTopic::Clear),
        "-h" | "--help" | "help" => Ok(HelpTopic::Root),
        other => Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("unknown help topic: {other}"),
        )),
    }
}

fn command_observe(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    let options = Options::parse(args)?;
    reject_act_only_options_for_observe(&options)?;
    let capture = capture_backend(&options.backend);
    let result = observe(
        ObserveRequest {
            output_dir: options.output_dir,
            session: options.session,
            screen_id: options.screen_id,
            format: options.format,
            grid_overlay: options.grid_overlay,
            image_size_limit: options.image_size_limit,
        },
        capture.as_ref(),
    )?;
    print!("{}", render_observation(&result));
    Ok(())
}

fn command_act(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    let options = Options::parse(args)?;
    let input_markdown = match options.file {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };
    let capture = capture_backend(&options.backend);
    let input = input_backend(&options.backend);
    match act(
        ActRequest {
            output_dir: options.output_dir,
            session: options.session,
            default_screen_id: options.screen_id,
            input_markdown,
            dry_run: options.dry_run,
            format: options.format,
            grid_overlay: options.grid_overlay,
            image_size_limit: options.image_size_limit,
        },
        capture.as_ref(),
        input.as_ref(),
    ) {
        Ok(result) => {
            print!("{}", render_action_result(&result));
            Ok(())
        }
        Err(error) => {
            print!("{}", render_action_error(&error));
            process::exit(2);
        }
    }
}

fn command_doctor(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    let options = DoctorCommandOptions::parse(&args)?;
    let capture = capture_backend(&options.backend);
    let input = input_backend(&options.backend);
    let store = SessionStore::new(&options.output_dir);
    store.ensure_root()?;
    let (capture_status, screens) = match capture.list_screens() {
        Ok(screens) => (format!("OK backend={}", capture.name()), screens),
        Err(error) => (
            format!("ERROR backend={} reason={}", capture.name(), error.message),
            Vec::new(),
        ),
    };
    let capabilities = input.capabilities();
    let report = DoctorReport {
        os: env::consts::OS.to_string(),
        capture: capture_status,
        input: format!(
            "mouse={} keyboard={} backend={}",
            capabilities.can_mouse,
            capabilities.can_keyboard,
            input.name()
        ),
        output_dir: options.output_dir.display().to_string(),
        screens,
        ffmpeg: if has_ffmpeg() {
            "available".to_string()
        } else {
            "not_found".to_string()
        },
    };
    print!("{}", render_doctor(&report));
    Ok(())
}

fn command_recap(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    if args.is_empty() {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "recap requires a session path",
        ));
    }
    let session_path = PathBuf::from(&args[0]);
    let options = RecapCommandOptions::parse(&args[1..])?;
    let result = recap_with_options(&session_path, options.into_recap_options())?;
    print!("{}", render_recap(&result));
    Ok(())
}

fn command_clear(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    let options = ClearCommandOptions::parse(&args)?;
    validate_clear_target(&options.output_dir, options.force)?;
    let removed = if options.output_dir.exists() {
        fs::remove_dir_all(&options.output_dir)?;
        true
    } else {
        false
    };
    print!(
        "## FerrisGrid Clear\n- output_directory: {}\n- result: {}\n",
        options.output_dir.display(),
        if removed { "cleared" } else { "already_clean" }
    );
    Ok(())
}

fn reject_act_only_options_for_observe(options: &Options) -> ferrisgrid_core::Result<()> {
    if options.dry_run {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "--dry-run is only supported by ferrisgrid act",
        ));
    }
    if options.file.is_some() {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "--file is only supported by ferrisgrid act",
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct ClearCommandOptions {
    output_dir: PathBuf,
    force: bool,
}

impl ClearCommandOptions {
    fn parse(args: &[String]) -> ferrisgrid_core::Result<Self> {
        let mut options = Self {
            output_dir: env::var("FERRISGRID_OUTPUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".ferrisgrid")),
            force: false,
        };
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-dir" => {
                    index += 1;
                    options.output_dir = PathBuf::from(value(args, index, "--output-dir")?);
                }
                "--force" => {
                    options.force = true;
                }
                other => {
                    return Err(ferrisgrid_core::FerrisError::new(
                        ferrisgrid_core::ErrorKind::Protocol,
                        format!("unknown clear flag: {other}"),
                    ));
                }
            }
            index += 1;
        }
        Ok(options)
    }
}

#[derive(Debug)]
struct DoctorCommandOptions {
    output_dir: PathBuf,
    backend: String,
}

impl DoctorCommandOptions {
    fn parse(args: &[String]) -> ferrisgrid_core::Result<Self> {
        let mut options = Self {
            output_dir: env::var("FERRISGRID_OUTPUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".ferrisgrid")),
            backend: env::var("FERRISGRID_BACKEND").unwrap_or_else(|_| "native".to_string()),
        };
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-dir" => {
                    index += 1;
                    options.output_dir = PathBuf::from(value(args, index, "--output-dir")?);
                }
                "--backend" => {
                    index += 1;
                    options.backend = value(args, index, "--backend")?.to_string();
                }
                other => {
                    return Err(ferrisgrid_core::FerrisError::new(
                        ferrisgrid_core::ErrorKind::Protocol,
                        format!("unknown doctor flag: {other}"),
                    ));
                }
            }
            index += 1;
        }
        Ok(options)
    }
}

#[derive(Debug, Default)]
struct RecapCommandOptions {
    video: Option<VideoFormat>,
    framerate: u32,
}

impl RecapCommandOptions {
    fn parse(args: &[String]) -> ferrisgrid_core::Result<Self> {
        let mut options = Self {
            video: None,
            framerate: 2,
        };
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--video" => {
                    index += 1;
                    options.video = Some(VideoFormat::parse(value(args, index, "--video")?)?);
                }
                "--framerate" | "--fps" => {
                    let flag = args[index].clone();
                    index += 1;
                    options.framerate = parse_framerate(value(args, index, &flag)?)?;
                }
                other => {
                    return Err(ferrisgrid_core::FerrisError::new(
                        ferrisgrid_core::ErrorKind::Protocol,
                        format!("unknown recap flag: {other}"),
                    ));
                }
            }
            index += 1;
        }
        Ok(options)
    }

    fn into_recap_options(self) -> RecapOptions {
        RecapOptions {
            video: self.video,
            framerate: self.framerate,
        }
    }
}

#[derive(Debug)]
struct Options {
    output_dir: PathBuf,
    session: Option<String>,
    screen_id: Option<String>,
    format: ImageFormat,
    grid_overlay: bool,
    image_size_limit: ImageSizeLimit,
    backend: String,
    dry_run: bool,
    file: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> ferrisgrid_core::Result<Self> {
        let mut options = Self {
            output_dir: env::var("FERRISGRID_OUTPUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".ferrisgrid")),
            session: None,
            screen_id: env::var("FERRISGRID_DEFAULT_SCREEN_ID").ok(),
            format: ImageFormat::Jpg,
            grid_overlay: true,
            image_size_limit: match env::var("FERRISGRID_MAX_IMAGE_EDGE") {
                Ok(value) => parse_max_image_edge(&value)?,
                Err(_) => balanced_image_size_limit(),
            },
            backend: env::var("FERRISGRID_BACKEND").unwrap_or_else(|_| "native".to_string()),
            dry_run: false,
            file: None,
        };
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-dir" => {
                    index += 1;
                    options.output_dir = PathBuf::from(value(&args, index, "--output-dir")?);
                }
                "--session" => {
                    index += 1;
                    options.session = Some(value(&args, index, "--session")?.to_string());
                }
                "--screen-id" => {
                    index += 1;
                    options.screen_id = Some(value(&args, index, "--screen-id")?.to_string());
                }
                "--format" => {
                    index += 1;
                    options.format = value(&args, index, "--format")?.parse()?;
                }
                "--grid-overlay" => {
                    index += 1;
                    options.grid_overlay = parse_bool(value(&args, index, "--grid-overlay")?)?;
                }
                "--max-image-edge" => {
                    let flag = args[index].clone();
                    index += 1;
                    options.image_size_limit = parse_max_image_edge(value(&args, index, &flag)?)?;
                }
                "--resolution" => {
                    let flag = args[index].clone();
                    index += 1;
                    options.image_size_limit =
                        parse_resolution(value(&args, index, &flag)?, &flag)?;
                }
                "--no-downsample" => {
                    options.image_size_limit = ImageSizeLimit::Native;
                }
                "--backend" => {
                    index += 1;
                    options.backend = value(&args, index, "--backend")?.to_string();
                }
                "--dry-run" => {
                    options.dry_run = true;
                }
                "--file" => {
                    index += 1;
                    options.file = Some(PathBuf::from(value(&args, index, "--file")?));
                }
                other => {
                    return Err(ferrisgrid_core::FerrisError::new(
                        ferrisgrid_core::ErrorKind::Protocol,
                        format!("unknown flag: {other}"),
                    ));
                }
            }
            index += 1;
        }
        Ok(options)
    }
}

fn value<'a>(args: &'a [String], index: usize, flag: &str) -> ferrisgrid_core::Result<&'a str> {
    args.get(index).map(String::as_str).ok_or_else(|| {
        ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("{flag} requires a value"),
        )
    })
}

fn parse_bool(value: &str) -> ferrisgrid_core::Result<bool> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        other => Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("expected boolean, got {other}"),
        )),
    }
}

fn balanced_image_size_limit() -> ImageSizeLimit {
    ImageSizeLimit::Adaptive {
        min_long_edge: BALANCED_MIN_LONG_EDGE,
        min_short_edge: BALANCED_MIN_SHORT_EDGE,
    }
}

fn parse_max_image_edge(value: &str) -> ferrisgrid_core::Result<ImageSizeLimit> {
    match value {
        "native" | "none" | "off" | "0" => Ok(ImageSizeLimit::Native),
        other => {
            let edge = other.parse::<u32>().map_err(|_| {
                ferrisgrid_core::FerrisError::new(
                    ferrisgrid_core::ErrorKind::Protocol,
                    format!("expected max image edge pixels or native, got {other}"),
                )
            })?;
            if edge < 320 {
                return Err(ferrisgrid_core::FerrisError::new(
                    ferrisgrid_core::ErrorKind::Protocol,
                    "max image edge must be at least 320 pixels",
                ));
            }
            Ok(ImageSizeLimit::FixedMaxEdge(edge))
        }
    }
}

fn parse_resolution(value: &str, flag: &str) -> ferrisgrid_core::Result<ImageSizeLimit> {
    match value {
        "fast" => Ok(ImageSizeLimit::FixedMaxEdge(FAST_IMAGE_EDGE)),
        "balanced" => Ok(balanced_image_size_limit()),
        "detail" => Ok(ImageSizeLimit::FixedMaxEdge(DETAIL_IMAGE_EDGE)),
        "native" => Ok(ImageSizeLimit::Native),
        _ => parse_max_image_edge(value).map_err(|_| {
            ferrisgrid_core::FerrisError::new(
                ferrisgrid_core::ErrorKind::Protocol,
                format!("{flag} must be fast, balanced, detail, native, or a max image edge"),
            )
        }),
    }
}

fn parse_framerate(value: &str) -> ferrisgrid_core::Result<u32> {
    let framerate = value.parse::<u32>().map_err(|_| {
        ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("expected framerate as a positive integer, got {value}"),
        )
    })?;
    if framerate == 0 {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "framerate must be greater than 0",
        ));
    }
    Ok(framerate)
}

fn validate_clear_target(path: &Path, force: bool) -> ferrisgrid_core::Result<()> {
    if path.as_os_str().is_empty() || path == Path::new(".") || path == Path::new("/") {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "refusing to clear an unsafe output directory",
        ));
    }
    let is_default_named_dir = path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == ".ferrisgrid");
    if !is_default_named_dir && !force {
        return Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            "refusing to clear a custom output directory without --force",
        ));
    }
    Ok(())
}

fn has_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn print_help(topic: HelpTopic) {
    print!("{}", help_text(topic));
}

fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::Root => ROOT_HELP,
        HelpTopic::Observe => OBSERVE_HELP,
        HelpTopic::Act => ACT_HELP,
        HelpTopic::Doctor => DOCTOR_HELP,
        HelpTopic::Recap => RECAP_HELP,
        HelpTopic::Clear => CLEAR_HELP,
    }
}

const ROOT_HELP: &str = concat!(
    "FerrisGrid\n",
    "\n",
    "Single-step visual computer control for local agents.\n",
    "\n",
    "Agent loop:\n",
    "  1. Run `ferrisgrid observe`.\n",
    "  2. Inspect the screenshot path and coordinate metadata from stdout.\n",
    "  3. Write one compact Markdown action file.\n",
    "  4. Run `ferrisgrid act --file .ferrisgrid/action.md`.\n",
    "  5. Inspect the returned post-action screenshot path and repeat.\n",
    "\n",
    "Docs:\n",
    "  ",
    docs_url!(),
    "\n",
    "  Commands: ",
    docs_url!(),
    "commands/\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid observe [options]\n",
    "  ferrisgrid act [options] [--file action.md]\n",
    "  ferrisgrid doctor [options]\n",
    "  ferrisgrid recap <session_path> [options]\n",
    "  ferrisgrid clear [options]\n",
    "  ferrisgrid help [observe|act|doctor|recap|clear]\n",
    "\n",
    "Command help:\n",
    "  ferrisgrid observe --help\n",
    "  ferrisgrid act --help\n",
    "  ferrisgrid doctor --help\n",
    "  ferrisgrid recap --help\n",
    "  ferrisgrid clear --help\n",
    "\n",
    "Common output/capture options:\n",
    "  --output-dir <path>            Trace root. Default: .ferrisgrid\n",
    "  --backend <name>               native, native-linux-x11, native-macos, fake.\n",
    "  --format <jpg|png>             Screenshot format. Default: jpg.\n",
    "  --grid-overlay <true|false>    Stamp coordinate grid on screenshots. Default: true.\n",
    "  --resolution <preset|pixels>   fast, balanced, detail, native, or max edge pixels.\n",
    "  --max-image-edge <pixels|native>\n",
    "  --no-downsample                Keep native screenshot dimensions.\n",
    "\n",
    "Environment defaults:\n",
    "  FERRISGRID_OUTPUT_DIR          Default trace root.\n",
    "  FERRISGRID_DEFAULT_SCREEN_ID   Default observe screen and act pointer-action target.\n",
    "  FERRISGRID_MAX_IMAGE_EDGE      Default max edge or native.\n",
    "  FERRISGRID_BACKEND             Default backend name.\n",
    "\n",
    "Session behavior:\n",
    "  - observe creates a new session or creates/resumes `--session <name-or-path>`.\n",
    "  - act uses `--session <name-or-path>` or the latest existing session. Run observe first.\n",
    "\n",
    "Coordinate protocol:\n",
    "  - stdout declares coordinate_mode, range, screen IDs, screenshot paths, and metadata paths.\n",
    "  - Use normalized-1000 coordinates: x=0 y=0 is top-left, x=1000 y=1000 is bottom-right.\n",
    "  - Coordinates are screen-local. Include screen_id when multiple screens are listed.\n",
    "  - For clean screenshots, pass `--grid-overlay false` to observe and act.\n",
    "\n",
    "Action Markdown summary:\n",
    "  status: action\n",
    "  action: click | double_click | right_click | move_mouse | drag | scroll | type | press_key | hotkey | wait\n",
    "  screen_id: screen-1            For pointer actions; required on multi-screen systems unless act --screen-id is set.\n",
    "  wait_after_ms: 500             Optional. Max 30000. Captures after waiting.\n",
    "\n",
    "Minimal action file:\n",
    "  status: action\n",
    "  action: click\n",
    "  screen_id: screen-1\n",
    "  x: 500\n",
    "  y: 500\n",
    "  button: left\n",
    "  wait_after_ms: 500\n",
    "\n",
    "Examples:\n",
    "  ferrisgrid doctor\n",
    "  ferrisgrid observe --grid-overlay false --resolution detail\n",
    "  ferrisgrid act --file .ferrisgrid/action.md --grid-overlay false\n",
    "  ferrisgrid recap .ferrisgrid/sessions/<session_id> --video mp4\n",
);

const OBSERVE_HELP: &str = concat!(
    "FerrisGrid observe\n",
    "\n",
    "Capture the current desktop state and print agent-readable Markdown with screenshot paths,\n",
    "screen metadata, and normalized coordinate mapping.\n",
    "\n",
    "Docs: ",
    docs_url!(),
    "commands/observe.html\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid observe [options]\n",
    "\n",
    "Options:\n",
    "  --output-dir <path>            Trace root. Default: .ferrisgrid\n",
    "  --session <name-or-path>       Create or resume a named session.\n",
    "  --screen-id <screen-id>        Capture only one screen, for example screen-1.\n",
    "  --backend <name>               native, native-linux-x11, native-macos, fake.\n",
    "  --format <jpg|png>             Screenshot format. Default: jpg.\n",
    "  --grid-overlay <true|false>    Stamp coordinate grid on screenshots. Default: true.\n",
    "  --resolution <preset|pixels>   fast, balanced, detail, native, or max edge pixels.\n",
    "  --max-image-edge <pixels|native>\n",
    "  --no-downsample                Keep native screenshot dimensions.\n",
    "  -h, --help                     Show this help.\n",
    "\n",
    "Output contract:\n",
    "  - session: local session directory.\n",
    "  - step: frame number within the session.\n",
    "  - coordinate_mode: normalized-1000.\n",
    "  - screen lines include screen_id, image size, native size, screenshot path, and metadata path.\n",
    "  - map lines describe normalized-to-image and normalized-to-native coordinate formulas.\n",
    "\n",
    "Agent notes:\n",
    "  - Read the returned screenshot file before choosing an action.\n",
    "  - Use `--grid-overlay false` when grid lines would obscure target UI or text.\n",
    "  - If multiple screens are listed, include screen_id in pointer actions.\n",
    "\n",
    "Examples:\n",
    "  ferrisgrid observe\n",
    "  ferrisgrid observe --screen-id screen-1 --grid-overlay false\n",
    "  ferrisgrid observe --resolution detail --format png\n",
);

const ACT_HELP: &str = concat!(
    "FerrisGrid act\n",
    "\n",
    "Execute exactly one compact Markdown action, then capture and print the updated screen state.\n",
    "\n",
    "Docs: ",
    docs_url!(),
    "commands/act.html\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid act --file action.md [options]\n",
    "  ferrisgrid act [options] < action.md\n",
    "\n",
    "Options:\n",
    "  --file <path>                  Read compact Markdown action from this file. Otherwise stdin.\n",
    "  --dry-run                      Validate and parse without emitting OS input.\n",
    "  --output-dir <path>            Trace root. Default: .ferrisgrid\n",
    "  --session <name-or-path>       Use an existing session. Default: latest session.\n",
    "  --screen-id <screen-id>        Default target for pointer actions that omit screen_id.\n",
    "  --backend <name>               native, native-linux-x11, native-macos, fake.\n",
    "  --format <jpg|png>             Post-action screenshot format. Default: jpg.\n",
    "  --grid-overlay <true|false>    Stamp coordinate grid on post-action screenshots. Default: true.\n",
    "  --resolution <preset|pixels>   fast, balanced, detail, native, or max edge pixels.\n",
    "  --max-image-edge <pixels|native>\n",
    "  --no-downsample                Keep native screenshot dimensions.\n",
    "  -h, --help                     Show this help.\n",
    "\n",
    "Required action format:\n",
    "  Use compact Markdown key/value lines. JSON is rejected.\n",
    "  For active actions, status defaults to action when omitted.\n",
    "  Terminal statuses do not execute input:\n",
    "    status: done\n",
    "    reason: task complete\n",
    "  Terminal statuses return screens: 0 and no post-action screenshot.\n",
    "\n",
    "Common fields:\n",
    "  status: action | done | fail\n",
    "  action: click | double_click | right_click | move_mouse | drag | scroll | type | press_key | hotkey | wait\n",
    "  screen_id: screen-1            Pointer actions only. Required on multi-screen systems unless --screen-id is set.\n",
    "  wait_after_ms: 500             Optional for action status. Max 30000.\n",
    "  confidence: 0.82               Optional agent-supplied confidence.\n",
    "  reason: short note             Optional agent-supplied reason.\n",
    "\n",
    "Pointer examples:\n",
    "  status: action\n",
    "  action: click\n",
    "  screen_id: screen-1\n",
    "  x: 742\n",
    "  y: 611\n",
    "  button: left\n",
    "  wait_after_ms: 700\n",
    "\n",
    "  action: drag\n",
    "  screen_id: screen-1\n",
    "  from_x: 450\n",
    "  from_y: 500\n",
    "  to_x: 620\n",
    "  to_y: 500\n",
    "  duration_ms: 450\n",
    "\n",
    "Scroll example:\n",
    "  action: scroll\n",
    "  screen_id: screen-1\n",
    "  x: 500\n",
    "  y: 500\n",
    "  delta_y: -720\n",
    "\n",
    "  action: scroll\n",
    "  delta_y: -720\n",
    "\n",
    "Non-screen examples:\n",
    "  action: wait\n",
    "  duration_ms: 1000\n",
    "\n",
    "Keyboard examples:\n",
    "  action: type\n",
    "  text: hello\n",
    "\n",
    "  action: press_key\n",
    "  key: enter\n",
    "\n",
    "  action: hotkey\n",
    "  keys: Cmd+Space\n",
    "\n",
    "Safety limits:\n",
    "  - Coordinates must be 0..1000.\n",
    "  - Scroll x and y must be supplied together or both omitted.\n",
    "  - Scroll delta_x and delta_y absolute values must be <= 2000.\n",
    "  - Drag duration_ms must be <= 5000.\n",
    "  - Type text is limited to 500 characters.\n",
    "  - Hotkeys may contain 1 to 4 keys.\n",
    "  - wait and wait_after_ms are limited to 30000 ms.\n",
    "\n",
    "Examples:\n",
    "  ferrisgrid act --file .ferrisgrid/action.md\n",
    "  ferrisgrid act --file .ferrisgrid/action.md --grid-overlay false\n",
    "  ferrisgrid act --file .ferrisgrid/action.md --dry-run\n",
);

const DOCTOR_HELP: &str = concat!(
    "FerrisGrid doctor\n",
    "\n",
    "Check capture, input, screen discovery, output directory, and ffmpeg availability.\n",
    "\n",
    "Docs: ",
    docs_url!(),
    "commands/doctor.html\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid doctor [options]\n",
    "\n",
    "Options:\n",
    "  --output-dir <path>            Trace root to create/check. Default: .ferrisgrid\n",
    "  --backend <name>               native, native-linux-x11, native-macos, fake.\n",
    "  -h, --help                     Show this help.\n",
    "\n",
    "Agent notes:\n",
    "  - On macOS, capture/input may require Screen Recording and Accessibility permissions.\n",
    "  - On Linux/X11, DISPLAY must be set for native-linux-x11.\n",
    "  - `screens: 0` or capture errors mean observe cannot proceed until the desktop backend works.\n",
);

const RECAP_HELP: &str = concat!(
    "FerrisGrid recap\n",
    "\n",
    "Generate human-review artifacts from an existing local session directory.\n",
    "\n",
    "Docs: ",
    docs_url!(),
    "commands/recap.html\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid recap <session_path> [options]\n",
    "\n",
    "Options:\n",
    "  --video mp4                    Also export an MP4 session video.\n",
    "  --framerate <fps>              Video frames per second. Default: 2.\n",
    "  --fps <fps>                    Alias for --framerate.\n",
    "  -h, --help                     Show this help.\n",
    "\n",
    "Example:\n",
    "  ferrisgrid recap .ferrisgrid/sessions/<session_id> --video mp4 --framerate 2\n",
);

const CLEAR_HELP: &str = concat!(
    "FerrisGrid clear\n",
    "\n",
    "Remove a FerrisGrid output directory.\n",
    "\n",
    "Docs: ",
    docs_url!(),
    "commands/clear.html\n",
    "\n",
    "Usage:\n",
    "  ferrisgrid clear [options]\n",
    "\n",
    "Options:\n",
    "  --output-dir <path>            Directory to remove. Default: .ferrisgrid\n",
    "  --force                        Required for custom output directories.\n",
    "  -h, --help                     Show this help.\n",
    "\n",
    "Safety:\n",
    "  - Refuses empty paths, current directory, and filesystem root.\n",
    "  - Refuses custom output directories unless --force is present.\n",
);

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisgrid_capture::FakeCaptureBackend;
    use std::sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn parse_options(args: &[&str]) -> Options {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_option_env();
        Options::parse(args.iter().map(|value| value.to_string()).collect()).unwrap()
    }

    fn parse_options_with_env(args: &[&str]) -> Options {
        Options::parse(args.iter().map(|value| value.to_string()).collect()).unwrap()
    }

    fn clear_option_env() {
        unsafe {
            env::remove_var("FERRISGRID_MAX_IMAGE_EDGE");
            env::remove_var("FERRISGRID_OUTPUT_DIR");
            env::remove_var("FERRISGRID_DEFAULT_SCREEN_ID");
            env::remove_var("FERRISGRID_BACKEND");
        }
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!(
            "ferrisgrid-cli-test-{name}-{}-{nonce}-{counter}",
            process::id()
        ))
    }

    #[test]
    fn root_help_is_agent_self_documenting() {
        let help = help_text(HelpTopic::Root);

        assert!(help.contains("Agent loop:"));
        assert!(help.contains("Coordinate protocol:"));
        assert!(help.contains("Action Markdown summary:"));
        assert!(help.contains("https://brunov21.github.io/FerrisGrid-CLI/"));
    }

    #[test]
    fn act_help_documents_action_schema_and_overlay_flag() {
        let help = help_text(HelpTopic::Act);

        assert!(help.contains("--grid-overlay <true|false>"));
        assert!(help.contains("--screen-id <screen-id>"));
        assert!(help.contains("JSON is rejected"));
        assert!(help.contains("action: click | double_click"));
        assert!(help.contains("wait_after_ms"));
        assert!(help.contains("keys: Cmd+Space"));
    }

    #[test]
    fn parses_help_topics() {
        assert_eq!(parse_help_topic("observe").unwrap(), HelpTopic::Observe);
        assert_eq!(parse_help_topic("act").unwrap(), HelpTopic::Act);
        assert_eq!(parse_help_topic("--help").unwrap(), HelpTopic::Root);

        let error = parse_help_topic("missing").unwrap_err();
        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("unknown help topic"));
    }

    #[test]
    fn recognizes_command_help_requests() {
        assert!(is_help_request(&["--help".to_string()]));
        assert!(is_help_request(&["-h".to_string()]));
        assert!(is_help_request(&["help".to_string()]));
        assert!(!is_help_request(&[
            "--grid-overlay".to_string(),
            "false".to_string()
        ]));
    }

    #[test]
    fn observe_rejects_act_only_options() {
        let error = reject_act_only_options_for_observe(&parse_options(&["--dry-run"]))
            .expect_err("observe should reject --dry-run");

        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("--dry-run"));

        let error = reject_act_only_options_for_observe(&parse_options(&["--file", "action.md"]))
            .expect_err("observe should reject --file");

        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("--file"));
    }

    #[test]
    fn rejects_undocumented_legacy_grid_flags() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_option_env();

        let error = Options::parse(vec!["--grid-step".to_string(), "100".to_string()])
            .expect_err("legacy grid flags should not be silently ignored");

        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("unknown flag"));
    }

    #[test]
    fn doctor_parser_rejects_irrelevant_shared_flags() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_option_env();

        let error =
            DoctorCommandOptions::parse(&["--grid-overlay".to_string(), "false".to_string()])
                .expect_err("doctor should reject observe/act-only flags");

        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("unknown doctor flag"));
    }

    fn observe_fake_dimensions(image_size_limit: ImageSizeLimit) -> Vec<(u32, u32)> {
        let output_dir = temp_output_dir("observe-dimensions");
        let result = observe(
            ObserveRequest {
                output_dir: output_dir.clone(),
                session: None,
                screen_id: None,
                format: ImageFormat::Jpg,
                grid_overlay: true,
                image_size_limit,
            },
            &FakeCaptureBackend::new(),
        )
        .unwrap();
        let dimensions = result
            .screens
            .iter()
            .map(|screen| (screen.image_width, screen.image_height))
            .collect();
        let _ = fs::remove_dir_all(output_dir);
        dimensions
    }

    #[test]
    fn default_resolution_is_adaptive_balanced() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_option_env();

        assert_eq!(
            parse_options_with_env(&[]).image_size_limit,
            balanced_image_size_limit()
        );
    }

    #[test]
    fn parses_resolution_presets() {
        assert_eq!(
            parse_options(&["--resolution", "fast"]).image_size_limit,
            ImageSizeLimit::FixedMaxEdge(FAST_IMAGE_EDGE)
        );
        assert_eq!(
            parse_options(&["--resolution", "balanced"]).image_size_limit,
            balanced_image_size_limit()
        );
        assert_eq!(
            parse_options(&["--resolution", "detail"]).image_size_limit,
            ImageSizeLimit::FixedMaxEdge(DETAIL_IMAGE_EDGE)
        );
        assert_eq!(
            parse_options(&["--resolution", "native"]).image_size_limit,
            ImageSizeLimit::Native
        );
    }

    #[test]
    fn parses_exact_max_image_edge() {
        assert_eq!(
            parse_options(&["--max-image-edge", "960"]).image_size_limit,
            ImageSizeLimit::FixedMaxEdge(960)
        );
    }

    #[test]
    fn env_can_override_default_max_image_edge() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var("FERRISGRID_MAX_IMAGE_EDGE", "960");
        }
        let options = parse_options_with_env(&[]);
        unsafe {
            env::remove_var("FERRISGRID_MAX_IMAGE_EDGE");
        }

        assert_eq!(options.image_size_limit, ImageSizeLimit::FixedMaxEdge(960));
    }

    #[test]
    fn later_resolution_flag_wins() {
        assert_eq!(
            parse_options(&["--resolution", "fast", "--max-image-edge", "960"]).image_size_limit,
            ImageSizeLimit::FixedMaxEdge(960)
        );
        assert_eq!(
            parse_options(&["--max-image-edge", "960", "--resolution", "native"]).image_size_limit,
            ImageSizeLimit::Native
        );
        assert_eq!(
            parse_options(&["--resolution", "detail", "--no-downsample"]).image_size_limit,
            ImageSizeLimit::Native
        );
    }

    #[test]
    fn rejects_invalid_resolution_preset() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_option_env();
        let error =
            Options::parse(vec!["--resolution".to_string(), "tiny".to_string()]).unwrap_err();

        assert_eq!(error.kind, ferrisgrid_core::ErrorKind::Protocol);
        assert!(error.message.contains("fast, balanced, detail, native"));
    }

    #[test]
    fn fake_backend_uses_fast_resolution_dimensions() {
        let dimensions = observe_fake_dimensions(ImageSizeLimit::FixedMaxEdge(FAST_IMAGE_EDGE));

        assert_eq!(dimensions, vec![(800, 520), (800, 450)]);
    }

    #[test]
    fn fake_backend_uses_adaptive_balanced_resolution_dimensions() {
        let dimensions = observe_fake_dimensions(balanced_image_size_limit());

        assert_eq!(dimensions, vec![(800, 520), (889, 500)]);
    }

    #[test]
    fn fake_backend_uses_detail_resolution_dimensions() {
        let dimensions = observe_fake_dimensions(ImageSizeLimit::FixedMaxEdge(DETAIL_IMAGE_EDGE));

        assert_eq!(dimensions, vec![(1920, 1247), (1920, 1080)]);
    }

    #[test]
    fn fake_backend_uses_native_resolution_dimensions() {
        let dimensions = observe_fake_dimensions(ImageSizeLimit::Native);

        assert_eq!(dimensions, vec![(3024, 1964), (2560, 1440)]);
    }
}
