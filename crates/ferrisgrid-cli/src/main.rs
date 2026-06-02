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
        print_help();
        return Ok(());
    }
    let command = args.remove(0);
    match command.as_str() {
        "observe" => command_observe(args),
        "act" => command_act(args),
        "doctor" => command_doctor(args),
        "recap" => command_recap(args),
        "clear" => command_clear(args),
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(ferrisgrid_core::FerrisError::new(
            ferrisgrid_core::ErrorKind::Protocol,
            format!("unknown command: {other}"),
        )),
    }
}

fn command_observe(args: Vec<String>) -> ferrisgrid_core::Result<()> {
    let options = Options::parse(args)?;
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
            input_markdown,
            dry_run: options.dry_run,
            format: options.format,
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
    let options = Options::parse(args)?;
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
                "--quality" | "--grid-step" | "--grid-labels" | "--grid-opacity" => {
                    index += 1;
                    let _ = value(&args, index, args[index - 1].as_str())?;
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

fn print_help() {
    println!(
        "FerrisGrid\n\nCommands:\n  ferrisgrid observe [--backend native|native-linux-x11|native-macos|fake] [--screen-id screen-1] [--grid-overlay true|false] [--resolution fast|balanced|detail|native] [--max-image-edge 800|native]\n  ferrisgrid act [--backend native|native-linux-x11|native-macos|fake] [--file action.md] [--dry-run] [--resolution fast|balanced|detail|native] [--max-image-edge 800|native]\n  ferrisgrid doctor [--backend native|native-linux-x11|native-macos|fake]\n  ferrisgrid recap <session_path> [--video mp4] [--framerate 2]\n  ferrisgrid clear [--output-dir .ferrisgrid] [--force]\n"
    );
}

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
