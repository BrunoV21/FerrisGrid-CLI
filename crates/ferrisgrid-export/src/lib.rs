use ferrisgrid_core::{ErrorKind, FerrisError, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct RecapResult {
    pub session_dir: PathBuf,
    pub recap_path: PathBuf,
    pub frame_count: usize,
}

pub fn recap(session_dir: &Path) -> Result<RecapResult> {
    if !session_dir.exists() {
        return Err(FerrisError::new(
            ErrorKind::Storage,
            format!("session path not found: {}", session_dir.display()),
        ));
    }
    let export_dir = session_dir.join("export");
    fs::create_dir_all(&export_dir)?;
    let frame_count = count_frame_dirs(session_dir)?;
    let recap_path = export_dir.join("recap.md");
    fs::write(
        &recap_path,
        format!(
            "## FerrisGrid Recap\n- session: {}\n- frames: {}\n- recap: {}\n",
            session_dir.display(),
            frame_count,
            recap_path.display()
        ),
    )?;
    Ok(RecapResult {
        session_dir: session_dir.to_path_buf(),
        recap_path,
        frame_count,
    })
}

pub fn render_recap(result: &RecapResult) -> String {
    format!(
        "## FerrisGrid Recap\n- session: {}\n- frames: {}\n- recap: {}\n",
        result.session_dir.display(),
        result.frame_count,
        result.recap_path.display()
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
