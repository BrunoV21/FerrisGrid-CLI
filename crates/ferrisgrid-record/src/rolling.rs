use ferrisgrid_core::{
    CaptureBackend, CaptureTarget, CapturedScreen, ErrorKind, FerrisError, ImageFormat,
    ImageSizeLimit, Result, ScreenInfo, SessionStore,
};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct CachedScreen {
    screen: ScreenInfo,
    image_width: u32,
    image_height: u32,
    extension: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub captured_at_ms: u64,
    screens: Vec<CachedScreen>,
}

#[derive(Default)]
struct RollingState {
    frames: VecDeque<FrameSnapshot>,
    last_error: Option<FerrisError>,
    fatal_error: Option<FerrisError>,
}

pub struct RollingCapture {
    state: Arc<Mutex<RollingState>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    temp_root: PathBuf,
}

impl RollingCapture {
    pub fn start(
        capture: Arc<dyn CaptureBackend>,
        format: ImageFormat,
        image_size_limit: ImageSizeLimit,
        fps: u32,
        expected_screens: &[ScreenInfo],
    ) -> Result<Self> {
        if fps == 0 || fps > 30 {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                "recording fps must be within 1..30",
            ));
        }
        let temp_root = std::env::temp_dir().join(format!(
            "ferrisgrid-record-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        fs::create_dir_all(&temp_root)?;
        let state = Arc::new(Mutex::new(RollingState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let thread_state = state.clone();
        let thread_stop = stop.clone();
        let thread_paused = paused.clone();
        let thread_root = temp_root.clone();
        let interval = Duration::from_millis((1000_u64 / fps as u64).max(1));
        let max_frames = (fps as usize * 6).max(8);
        let mut expected_fingerprints = expected_screens
            .iter()
            .map(|screen| screen.display_fingerprint.clone())
            .collect::<Vec<_>>();
        expected_fingerprints.sort();
        let worker = thread::spawn(move || {
            let mut frame_number = 0_u64;
            while !thread_stop.load(Ordering::Relaxed) {
                if thread_paused.load(Ordering::Relaxed) {
                    thread::sleep(interval);
                    continue;
                }
                let started = Instant::now();
                frame_number += 1;
                let frame_dir = thread_root.join(format!("{frame_number:08}"));
                let result = capture.capture(
                    CaptureTarget::All,
                    &frame_dir,
                    &format,
                    false,
                    image_size_limit,
                );
                match result.and_then(cache_captured_screens) {
                    Ok(screens) => {
                        let mut actual = screens
                            .iter()
                            .map(|screen| screen.screen.display_fingerprint.clone())
                            .collect::<Vec<_>>();
                        actual.sort();
                        if actual != expected_fingerprints {
                            if let Ok(mut state) = thread_state.lock() {
                                state.frames.clear();
                                state.fatal_error = Some(FerrisError::new(
                                    ErrorKind::Coordinate,
                                    "display topology changed while recording; the session was stopped before writing ambiguous coordinates",
                                ));
                            }
                            thread_stop.store(true, Ordering::Relaxed);
                            continue;
                        }
                        if let Ok(mut state) = thread_state.lock() {
                            state.frames.push_back(FrameSnapshot {
                                captured_at_ms: unix_millis(),
                                screens,
                            });
                            while state.frames.len() > max_frames {
                                state.frames.pop_front();
                            }
                            state.last_error = None;
                        }
                    }
                    Err(error) => {
                        if let Ok(mut state) = thread_state.lock() {
                            state.last_error = Some(error);
                        }
                    }
                }
                let _ = fs::remove_dir_all(&frame_dir);
                if let Some(remaining) = interval.checked_sub(started.elapsed()) {
                    thread::sleep(remaining);
                }
            }
        });
        Ok(Self {
            state,
            stop,
            paused,
            worker: Some(worker),
            temp_root,
        })
    }

    pub fn wait_initial(&self, timeout: Duration) -> Result<FrameSnapshot> {
        self.wait_for(|frames| frames.back().cloned(), timeout)
    }

    pub fn at_or_before(&self, at_ms: u64) -> Result<FrameSnapshot> {
        let state = self.state.lock().map_err(lock_error)?;
        if let Some(error) = &state.fatal_error {
            return Err(error.clone());
        }
        state
            .frames
            .iter()
            .rev()
            .find(|frame| frame.captured_at_ms <= at_ms)
            .cloned()
            .or_else(|| state.frames.front().cloned())
            .ok_or_else(|| capture_unavailable(&state))
    }

    pub fn latest(&self) -> Result<FrameSnapshot> {
        let state = self.state.lock().map_err(lock_error)?;
        if let Some(error) = &state.fatal_error {
            return Err(error.clone());
        }
        state
            .frames
            .back()
            .cloned()
            .ok_or_else(|| capture_unavailable(&state))
    }

    pub fn check_health(&self) -> Result<()> {
        let state = self.state.lock().map_err(lock_error)?;
        if let Some(error) = &state.fatal_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    }

    pub fn wait_after(&self, at_ms: u64, timeout: Duration) -> Result<FrameSnapshot> {
        self.wait_for(
            |frames| {
                frames
                    .iter()
                    .find(|frame| frame.captured_at_ms >= at_ms)
                    .cloned()
            },
            timeout,
        )
    }

    fn wait_for(
        &self,
        select: impl Fn(&VecDeque<FrameSnapshot>) -> Option<FrameSnapshot>,
        timeout: Duration,
    ) -> Result<FrameSnapshot> {
        let started = Instant::now();
        loop {
            {
                let state = self.state.lock().map_err(lock_error)?;
                if let Some(error) = &state.fatal_error {
                    return Err(error.clone());
                }
                if let Some(frame) = select(&state.frames) {
                    return Ok(frame);
                }
                if started.elapsed() >= timeout {
                    return Err(capture_unavailable(&state));
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = fs::remove_dir_all(&self.temp_root);
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
        if !paused && let Ok(mut state) = self.state.lock() {
            state.frames.clear();
        }
    }
}

impl Drop for RollingCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

impl FrameSnapshot {
    pub fn persist(
        &self,
        store: &SessionStore,
        session_dir: &std::path::Path,
        frame_id: u32,
    ) -> Result<Vec<CapturedScreen>> {
        let frame_dir = store.frame_dir(session_dir, frame_id)?;
        let mut captured = Vec::new();
        for cached in &self.screens {
            let screenshot_path =
                frame_dir.join(format!("{}.{}", cached.screen.screen_id, cached.extension));
            fs::write(&screenshot_path, &cached.bytes)?;
            let metadata_path = frame_dir.join(format!("{}.meta.md", cached.screen.screen_id));
            fs::write(
                &metadata_path,
                render_metadata(
                    &cached.screen,
                    &screenshot_path,
                    cached.image_width,
                    cached.image_height,
                    self.captured_at_ms,
                ),
            )?;
            captured.push(CapturedScreen {
                screen: cached.screen.clone(),
                image_width: cached.image_width,
                image_height: cached.image_height,
                screenshot_path,
                metadata_path,
            });
        }
        Ok(captured)
    }
}

fn cache_captured_screens(captured: Vec<CapturedScreen>) -> Result<Vec<CachedScreen>> {
    captured
        .into_iter()
        .map(|captured| {
            let extension = captured
                .screenshot_path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("jpg")
                .to_string();
            Ok(CachedScreen {
                screen: captured.screen,
                image_width: captured.image_width,
                image_height: captured.image_height,
                extension,
                bytes: fs::read(captured.screenshot_path)?,
            })
        })
        .collect()
}

fn render_metadata(
    screen: &ScreenInfo,
    screenshot_path: &std::path::Path,
    image_width: u32,
    image_height: u32,
    captured_at_ms: u64,
) -> String {
    format!(
        "## Screen Metadata\n- screen_id: {}\n- display_fingerprint: {}\n- name: {}\n- captured_at_unix_ms: {}\n- coordinate_mode: normalized-1000\n- coordinate_origin: top_left\n- coordinate_scope: screen_local\n- origin_x: {}\n- origin_y: {}\n- logical_width: {}\n- logical_height: {}\n- native_width: {}\n- native_height: {}\n- image_width: {}\n- image_height: {}\n- scale_factor: {}\n- screenshot: {}\n",
        screen.screen_id,
        screen.display_fingerprint,
        screen.name,
        captured_at_ms,
        screen.origin_x,
        screen.origin_y,
        screen.logical_width,
        screen.logical_height,
        screen.native_width,
        screen.native_height,
        image_width,
        image_height,
        screen.scale_factor,
        screenshot_path.display()
    )
}

fn capture_unavailable(state: &RollingState) -> FerrisError {
    state.last_error.clone().unwrap_or_else(|| {
        FerrisError::new(
            ErrorKind::Capture,
            "rolling capture did not produce a frame before the timeout",
        )
    })
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> FerrisError {
    FerrisError::new(
        ErrorKind::Storage,
        "rolling capture state lock was poisoned",
    )
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
