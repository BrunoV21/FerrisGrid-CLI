use ferrisgrid_core::{ErrorKind, FerrisError, Result};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct RecapResult {
    pub session_dir: PathBuf,
    pub recap_path: PathBuf,
    pub frame_count: usize,
    pub video_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    Mp4,
}

impl VideoFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "mp4" => Ok(Self::Mp4),
            other => Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("unsupported video format: {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RecapOptions {
    pub video: Option<VideoFormat>,
    pub framerate: u32,
}

pub fn recap(session_dir: &Path) -> Result<RecapResult> {
    recap_with_options(session_dir, RecapOptions::default())
}

pub fn recap_with_options(session_dir: &Path, mut options: RecapOptions) -> Result<RecapResult> {
    if !session_dir.exists() {
        return Err(FerrisError::new(
            ErrorKind::Storage,
            format!("session path not found: {}", session_dir.display()),
        ));
    }
    if options.framerate == 0 {
        options.framerate = 2;
    }
    let export_dir = session_dir.join("export");
    fs::create_dir_all(&export_dir)?;
    let frame_count = count_frame_dirs(session_dir)?;
    let recap_path = export_dir.join("recap.md");
    let video_path = match options.video {
        Some(VideoFormat::Mp4) => Some(export_mp4(session_dir, &export_dir, options.framerate)?),
        None => None,
    };
    let video_line = video_path
        .as_ref()
        .map(|path| format!("- video: {}\n", path.display()))
        .unwrap_or_default();
    fs::write(
        &recap_path,
        format!(
            "## FerrisGrid Recap\n- session: {}\n- frames: {}\n- recap: {}\n{}",
            session_dir.display(),
            frame_count,
            recap_path.display(),
            video_line
        ),
    )?;
    Ok(RecapResult {
        session_dir: session_dir.to_path_buf(),
        recap_path,
        frame_count,
        video_path,
    })
}

pub fn render_recap(result: &RecapResult) -> String {
    let video_line = result
        .video_path
        .as_ref()
        .map(|path| format!("- video: {}\n", path.display()))
        .unwrap_or_default();
    format!(
        "## FerrisGrid Recap\n- session: {}\n- frames: {}\n- recap: {}\n{}",
        result.session_dir.display(),
        result.frame_count,
        result.recap_path.display(),
        video_line
    )
}

fn count_frame_dirs(session_dir: &Path) -> Result<usize> {
    let frames = session_dir.join("frames");
    if !frames.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(frames)? {
        if entry?.file_type()?.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

fn export_mp4(session_dir: &Path, export_dir: &Path, framerate: u32) -> Result<PathBuf> {
    let frame_name = first_frame_file_name(session_dir)?;
    let input_pattern = session_dir
        .join("frames")
        .join("*")
        .join(&frame_name)
        .display()
        .to_string();
    let output_path = export_dir.join("session.mp4");
    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-framerate")
        .arg(framerate.to_string())
        .arg("-pattern_type")
        .arg("glob")
        .arg("-i")
        .arg(&input_pattern)
        .arg("-vf")
        .arg("scale=trunc(iw/2)*2:trunc(ih/2)*2")
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(&output_path)
        .output()
        .map_err(ffmpeg_error)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FerrisError::new(
            ErrorKind::Execution,
            format!("ffmpeg failed: {}", stderr.trim()),
        ));
    }
    Ok(output_path)
}

fn ffmpeg_error(error: io::Error) -> FerrisError {
    if error.kind() == io::ErrorKind::NotFound {
        return FerrisError::new(
            ErrorKind::Execution,
            "ffmpeg not found; install it with `brew install ffmpeg` or ensure ffmpeg is on PATH",
        );
    }
    FerrisError::new(
        ErrorKind::Execution,
        format!("failed to run ffmpeg: {error}"),
    )
}

fn first_frame_file_name(session_dir: &Path) -> Result<String> {
    let frames = session_dir.join("frames");
    if !frames.exists() {
        return Err(FerrisError::new(
            ErrorKind::Storage,
            format!("frames path not found: {}", frames.display()),
        ));
    }
    let mut frame_dirs = fs::read_dir(&frames)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => Some(entry.path()),
            _ => None,
        })
        .collect::<Vec<_>>();
    frame_dirs.sort();
    for frame_dir in frame_dirs {
        let mut files = fs::read_dir(frame_dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| match entry.file_type() {
                Ok(file_type) if file_type.is_file() => Some(entry.path()),
                _ => None,
            })
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let Some(extension) = file.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if matches!(extension, "jpg" | "jpeg" | "png") {
                let Some(file_name) = file.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                return Ok(file_name.to_string());
            }
        }
    }
    Err(FerrisError::new(
        ErrorKind::Storage,
        "no screenshot frames found for video export",
    ))
}
