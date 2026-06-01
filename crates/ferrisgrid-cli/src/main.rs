use ferrisgrid_capture::backend_by_name as capture_backend;
use ferrisgrid_core::{
    ActRequest, DoctorReport, ImageFormat, ObserveRequest, SessionStore, act, observe,
    render_action_error, render_action_result, render_doctor, render_observation,
};
use ferrisgrid_export::{recap, render_recap};
use ferrisgrid_input::backend_by_name as input_backend;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

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
    let result = recap(&PathBuf::from(&args[0]))?;
    print!("{}", render_recap(&result));
    Ok(())
}

#[derive(Debug)]
struct Options {
    output_dir: PathBuf,
    session: Option<String>,
    screen_id: Option<String>,
    format: ImageFormat,
    grid_overlay: bool,
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
                "--quality" | "--resolution" | "--grid-step" | "--grid-labels"
                | "--grid-opacity" => {
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

fn has_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn print_help() {
    println!(
        "FerrisGrid\n\nCommands:\n  ferrisgrid observe [--backend native|fake] [--screen-id screen-1] [--grid-overlay true|false]\n  ferrisgrid act [--backend native|fake] [--file action.md] [--dry-run]\n  ferrisgrid doctor [--backend native|fake]\n  ferrisgrid recap <session_path>\n"
    );
}
