use ferrisgrid_core::{
    CaptureBackend, CaptureTarget, CapturedScreen, ErrorKind, FerrisError, ImageFormat, Result,
    ScreenInfo,
};
use image::{DynamicImage, ImageReader, Rgba, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct FakeCaptureBackend;

impl FakeCaptureBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FakeCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBackend for FakeCaptureBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn list_screens(&self) -> Result<Vec<ScreenInfo>> {
        Ok(fake_screens())
    }

    fn capture(
        &self,
        target: CaptureTarget,
        frame_dir: &Path,
        format: &ImageFormat,
        grid_overlay: bool,
    ) -> Result<Vec<CapturedScreen>> {
        let screens = select_screens(fake_screens(), target)?;
        write_fake_captures(screens, frame_dir, format, grid_overlay)
    }
}

pub struct MacOsCaptureBackend;

impl CaptureBackend for MacOsCaptureBackend {
    fn name(&self) -> &'static str {
        "native-macos"
    }

    fn list_screens(&self) -> Result<Vec<ScreenInfo>> {
        Ok(native_screens()?
            .into_iter()
            .map(|screen| screen.info)
            .collect())
    }

    fn capture(
        &self,
        target: CaptureTarget,
        frame_dir: &Path,
        format: &ImageFormat,
        grid_overlay: bool,
    ) -> Result<Vec<CapturedScreen>> {
        #[cfg(target_os = "macos")]
        {
            let screens = select_native_screens(native_screens()?, target)?;
            fs::create_dir_all(frame_dir)?;
            let mut captured = Vec::new();
            for screen in screens {
                let screenshot_path =
                    frame_dir.join(format!("{}.{}", screen.info.screen_id, format.extension()));
                capture_macos_display(screen.capture_display_index, &screenshot_path, format)?;
                if grid_overlay {
                    apply_grid_overlay(&screenshot_path)?;
                }
                let (image_width, image_height) = image_dimensions(&screenshot_path)
                    .unwrap_or((screen.info.native_width, screen.info.native_height));
                let metadata_path = write_metadata(
                    frame_dir,
                    &screen.info,
                    &screenshot_path,
                    image_width,
                    image_height,
                )?;
                captured.push(CapturedScreen {
                    image_width,
                    image_height,
                    screen: screen.info,
                    screenshot_path,
                    metadata_path,
                });
            }
            Ok(captured)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (target, frame_dir, format, grid_overlay);
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native backend is currently implemented for macOS only; use --backend fake for local protocol tests",
            ))
        }
    }
}

pub fn backend_by_name(name: &str) -> Box<dyn CaptureBackend> {
    match name {
        "fake" => Box::new(FakeCaptureBackend),
        "native" | "macos" | "native-macos" => Box::new(MacOsCaptureBackend),
        _ => Box::new(MacOsCaptureBackend),
    }
}

#[derive(Clone)]
struct NativeScreen {
    info: ScreenInfo,
    capture_display_index: usize,
}

#[cfg(target_os = "macos")]
fn native_screens() -> Result<Vec<NativeScreen>> {
    let display_ids = display_ids()?;
    if display_ids.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "CoreGraphics returned no displays; run FerrisGrid from a logged-in desktop session with screen access",
        ));
    }

    let main_display = unsafe { CGMainDisplayID() };
    let mut screens = display_ids
        .iter()
        .enumerate()
        .map(|(index, display_id)| {
            let bounds = unsafe { CGDisplayBounds(*display_id) };
            let native_width = unsafe { CGDisplayPixelsWide(*display_id) as u32 };
            let native_height = unsafe { CGDisplayPixelsHigh(*display_id) as u32 };
            let scale_factor = if bounds.size.width > 0.0 {
                native_width as f64 / bounds.size.width
            } else {
                1.0
            } as f32;
            NativeScreen {
                info: ScreenInfo {
                    screen_id: String::new(),
                    name: if *display_id == main_display {
                        "Main Display".to_string()
                    } else {
                        format!("Display {}", index + 1)
                    },
                    is_primary: *display_id == main_display,
                    origin_x: bounds.origin.x.round() as i32,
                    origin_y: bounds.origin.y.round() as i32,
                    native_width,
                    native_height,
                    scale_factor,
                },
                // screencapture uses 1 for the main display and subsequent display numbers
                // for additional active displays.
                capture_display_index: index + 1,
            }
        })
        .collect::<Vec<_>>();

    screens.sort_by_key(|screen| {
        (
            !screen.info.is_primary,
            screen.info.origin_y,
            screen.info.origin_x,
        )
    });
    for (index, screen) in screens.iter_mut().enumerate() {
        screen.info.screen_id = format!("screen-{}", index + 1);
        screen.capture_display_index = index + 1;
        if !screen.info.is_primary {
            screen.info.name = format!("Display {}", index + 1);
        }
    }

    Ok(screens)
}

#[cfg(target_os = "macos")]
fn display_ids() -> Result<Vec<u32>> {
    let mut ids = [0_u32; 32];
    let mut count = 0_u32;
    let active_error =
        unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if active_error == 0 && count > 0 {
        return Ok(ids[..count as usize].to_vec());
    }

    count = 0;
    let online_error =
        unsafe { CGGetOnlineDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if online_error != 0 {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            format!(
                "CoreGraphics display discovery failed: active={active_error} online={online_error}"
            ),
        ));
    }
    Ok(ids[..count as usize].to_vec())
}

#[cfg(not(target_os = "macos"))]
fn native_screens() -> Result<Vec<NativeScreen>> {
    Err(FerrisError::new(
        ErrorKind::Platform,
        "native backend is currently implemented for macOS only; use --backend fake for local protocol tests",
    ))
}

#[cfg(target_os = "macos")]
fn capture_macos_display(
    display_index: usize,
    screenshot_path: &Path,
    format: &ImageFormat,
) -> Result<()> {
    let status = Command::new("/usr/sbin/screencapture")
        .arg("-x")
        .arg("-D")
        .arg(display_index.to_string())
        .arg("-t")
        .arg(format.extension())
        .arg(screenshot_path)
        .status()
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
    if !status.success() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "screencapture failed; check Screen Recording permission",
        ));
    }
    Ok(())
}

fn fake_screens() -> Vec<ScreenInfo> {
    vec![
        ScreenInfo {
            screen_id: "screen-1".to_string(),
            name: "Fake Primary".to_string(),
            is_primary: true,
            origin_x: 0,
            origin_y: 0,
            native_width: 3024,
            native_height: 1964,
            scale_factor: 2.0,
        },
        ScreenInfo {
            screen_id: "screen-2".to_string(),
            name: "Fake Secondary".to_string(),
            is_primary: false,
            origin_x: 3024,
            origin_y: 0,
            native_width: 2560,
            native_height: 1440,
            scale_factor: 1.0,
        },
    ]
}

fn select_screens(screens: Vec<ScreenInfo>, target: CaptureTarget) -> Result<Vec<ScreenInfo>> {
    match target {
        CaptureTarget::All => Ok(screens),
        CaptureTarget::Screen(id) => screens
            .into_iter()
            .filter(|screen| screen.screen_id == id)
            .collect::<Vec<_>>()
            .pipe(|selected| {
                if selected.is_empty() {
                    Err(FerrisError::new(
                        ErrorKind::Coordinate,
                        format!("unknown screen_id: {id}"),
                    ))
                } else {
                    Ok(selected)
                }
            }),
    }
}

fn select_native_screens(
    screens: Vec<NativeScreen>,
    target: CaptureTarget,
) -> Result<Vec<NativeScreen>> {
    match target {
        CaptureTarget::All => Ok(screens),
        CaptureTarget::Screen(id) => screens
            .into_iter()
            .filter(|screen| screen.info.screen_id == id)
            .collect::<Vec<_>>()
            .pipe(|selected| {
                if selected.is_empty() {
                    Err(FerrisError::new(
                        ErrorKind::Coordinate,
                        format!("unknown screen_id: {id}"),
                    ))
                } else {
                    Ok(selected)
                }
            }),
    }
}

fn write_fake_captures(
    screens: Vec<ScreenInfo>,
    frame_dir: &Path,
    format: &ImageFormat,
    grid_overlay: bool,
) -> Result<Vec<CapturedScreen>> {
    fs::create_dir_all(frame_dir)?;
    let mut captured = Vec::new();
    for screen in screens {
        let image_width = if screen.native_width > 1280 {
            1280
        } else {
            screen.native_width
        };
        let image_height = ((image_width as f64 / screen.native_width as f64)
            * screen.native_height as f64)
            .round() as u32;
        let screenshot_path =
            frame_dir.join(format!("{}.{}", screen.screen_id, format.extension()));
        write_placeholder_image(&screenshot_path, image_width, image_height)?;
        if grid_overlay {
            apply_grid_overlay(&screenshot_path)?;
        }
        let metadata_path = write_metadata(
            frame_dir,
            &screen,
            &screenshot_path,
            image_width,
            image_height,
        )?;
        captured.push(CapturedScreen {
            screen,
            image_width,
            image_height,
            screenshot_path,
            metadata_path,
        });
    }
    Ok(captured)
}

fn write_metadata(
    frame_dir: &Path,
    screen: &ScreenInfo,
    screenshot_path: &Path,
    image_width: u32,
    image_height: u32,
) -> Result<PathBuf> {
    let metadata_path = frame_dir.join(format!("{}.meta.md", screen.screen_id));
    fs::write(
        &metadata_path,
        format!(
            "## Screen Metadata\n- screen_id: {}\n- name: {}\n- origin_x: {}\n- origin_y: {}\n- native_width: {}\n- native_height: {}\n- image_width: {}\n- image_height: {}\n- scale_factor: {}\n- is_primary: {}\n- screenshot: {}\n",
            screen.screen_id,
            screen.name,
            screen.origin_x,
            screen.origin_y,
            screen.native_width,
            screen.native_height,
            image_width,
            image_height,
            screen.scale_factor,
            screen.is_primary,
            screenshot_path.display()
        ),
    )?;
    Ok(metadata_path)
}

fn write_placeholder_image(path: &Path, width: u32, height: u32) -> Result<()> {
    let width = width.max(1);
    let height = height.max(1);
    let mut image = RgbaImage::new(width, height);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let shade = ((x + y) % 255) as u8;
        *pixel = Rgba([shade, 80, 180, 255]);
    }
    DynamicImage::ImageRgba8(image)
        .save(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
    Ok(())
}

fn image_dimensions(path: &Path) -> Result<(u32, u32)> {
    image::image_dimensions(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))
}

fn apply_grid_overlay(path: &Path) -> Result<()> {
    let mut image = ImageReader::open(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
        .with_guessed_format()
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
        .decode()
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
        .to_rgba8();
    draw_grid(&mut image);
    DynamicImage::ImageRgba8(image)
        .save(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
    Ok(())
}

fn draw_grid(image: &mut RgbaImage) {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return;
    }

    let minor = Rgba([255, 210, 0, 155]);
    let major = Rgba([255, 90, 0, 210]);
    let axis = Rgba([0, 180, 255, 235]);

    for tick in (0..=1000).step_by(100) {
        let x = normalized_to_pixel(tick, width);
        let color = if tick == 0 || tick == 500 || tick == 1000 {
            major
        } else {
            minor
        };
        draw_vertical(image, x, color, 1);

        let y = normalized_to_pixel(tick, height);
        draw_horizontal(image, y, color, 1);
    }

    draw_vertical(image, normalized_to_pixel(500, width), axis, 2);
    draw_horizontal(image, normalized_to_pixel(500, height), axis, 2);

    for tick in (0..=1000).step_by(100) {
        let x = normalized_to_pixel(tick, width);
        let y = normalized_to_pixel(tick, height);
        draw_square(image, x, normalized_to_pixel(500, height), axis, 3);
        draw_square(image, normalized_to_pixel(500, width), y, axis, 3);
    }
}

fn normalized_to_pixel(value: u32, size: u32) -> u32 {
    (((value as f64 / 1000.0) * (size.saturating_sub(1)) as f64).round() as u32)
        .min(size.saturating_sub(1))
}

fn draw_vertical(image: &mut RgbaImage, x: u32, color: Rgba<u8>, radius: u32) {
    let (width, height) = image.dimensions();
    let start = x.saturating_sub(radius);
    let end = (x + radius).min(width.saturating_sub(1));
    for px in start..=end {
        for y in 0..height {
            blend_pixel(image, px, y, color);
        }
    }
}

fn draw_horizontal(image: &mut RgbaImage, y: u32, color: Rgba<u8>, radius: u32) {
    let (width, height) = image.dimensions();
    let start = y.saturating_sub(radius);
    let end = (y + radius).min(height.saturating_sub(1));
    for py in start..=end {
        for x in 0..width {
            blend_pixel(image, x, py, color);
        }
    }
}

fn draw_square(image: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>, radius: u32) {
    let (width, height) = image.dimensions();
    let x_start = x.saturating_sub(radius);
    let x_end = (x + radius).min(width.saturating_sub(1));
    let y_start = y.saturating_sub(radius);
    let y_end = (y + radius).min(height.saturating_sub(1));
    for px in x_start..=x_end {
        for py in y_start..=y_end {
            blend_pixel(image, px, py, color);
        }
    }
}

fn blend_pixel(image: &mut RgbaImage, x: u32, y: u32, overlay: Rgba<u8>) {
    let alpha = overlay[3] as f32 / 255.0;
    let pixel = image.get_pixel_mut(x, y);
    for channel in 0..3 {
        pixel[channel] = ((overlay[channel] as f32 * alpha)
            + (pixel[channel] as f32 * (1.0 - alpha)))
            .round() as u8;
    }
    pixel[3] = 255;
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    fn CGGetOnlineDisplayList(
        max_displays: u32,
        online_displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}
