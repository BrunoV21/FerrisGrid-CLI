use ferrisgrid_core::{
    CaptureBackend, CaptureTarget, CapturedScreen, ErrorKind, FerrisError, ImageFormat, Result,
    ScreenInfo,
};
use image::{DynamicImage, ImageReader, Rgba, RgbaImage, imageops::FilterType};
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
        max_image_edge: Option<u32>,
    ) -> Result<Vec<CapturedScreen>> {
        let screens = select_screens(fake_screens(), target)?;
        write_fake_captures(screens, frame_dir, format, grid_overlay, max_image_edge)
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
        max_image_edge: Option<u32>,
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
                downsample_image(&screenshot_path, max_image_edge)?;
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
            let _ = (target, frame_dir, format, grid_overlay, max_image_edge);
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native backend is currently implemented for macOS only; use --backend fake for local protocol tests",
            ))
        }
    }
}

pub struct LinuxCaptureBackend;

impl CaptureBackend for LinuxCaptureBackend {
    fn name(&self) -> &'static str {
        "native-linux-x11"
    }

    fn list_screens(&self) -> Result<Vec<ScreenInfo>> {
        linux_screens()
    }

    fn capture(
        &self,
        target: CaptureTarget,
        frame_dir: &Path,
        format: &ImageFormat,
        grid_overlay: bool,
        max_image_edge: Option<u32>,
    ) -> Result<Vec<CapturedScreen>> {
        #[cfg(target_os = "linux")]
        {
            let screens = select_screens(linux_screens()?, target)?;
            fs::create_dir_all(frame_dir)?;

            let root_path = frame_dir.join("root-capture.png");
            capture_linux_root(&root_path)?;
            let root_image = ImageReader::open(&root_path)
                .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
                .with_guessed_format()
                .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
                .decode()
                .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;

            let mut captured = Vec::new();
            for screen in screens {
                let x = screen.origin_x.max(0) as u32;
                let y = screen.origin_y.max(0) as u32;
                if x >= root_image.width() || y >= root_image.height() {
                    return Err(FerrisError::new(
                        ErrorKind::Capture,
                        format!(
                            "screen {} origin {},{} is outside root image {}x{}",
                            screen.screen_id,
                            screen.origin_x,
                            screen.origin_y,
                            root_image.width(),
                            root_image.height()
                        ),
                    ));
                }
                let crop_width = screen
                    .native_width
                    .min(root_image.width().saturating_sub(x))
                    .max(1);
                let crop_height = screen
                    .native_height
                    .min(root_image.height().saturating_sub(y))
                    .max(1);
                let screenshot_path =
                    frame_dir.join(format!("{}.{}", screen.screen_id, format.extension()));
                let cropped = root_image.crop_imm(x, y, crop_width, crop_height);
                cropped
                    .save(&screenshot_path)
                    .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
                downsample_image(&screenshot_path, max_image_edge)?;
                if grid_overlay {
                    apply_grid_overlay(&screenshot_path)?;
                }
                let (image_width, image_height) = image_dimensions(&screenshot_path)
                    .unwrap_or((screen.native_width, screen.native_height));
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
            let _ = fs::remove_file(root_path);
            Ok(captured)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (target, frame_dir, format, grid_overlay, max_image_edge);
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native Linux X11 capture is only available on Linux; use --backend native on this OS or --backend fake",
            ))
        }
    }
}

pub fn backend_by_name(name: &str) -> Box<dyn CaptureBackend> {
    match name {
        "fake" => Box::new(FakeCaptureBackend),
        "native" => native_backend(),
        "macos" | "native-macos" => Box::new(MacOsCaptureBackend),
        "linux" | "x11" | "native-linux" | "native-linux-x11" => Box::new(LinuxCaptureBackend),
        _ => native_backend(),
    }
}

fn native_backend() -> Box<dyn CaptureBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxCaptureBackend)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsCaptureBackend)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Box::new(MacOsCaptureBackend)
    }
}

#[derive(Clone)]
#[allow(dead_code)]
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

#[cfg(target_os = "linux")]
fn linux_screens() -> Result<Vec<ScreenInfo>> {
    let mut screens = run_output("xrandr", &["--query"])
        .map(|output| parse_xrandr_screens(&output))
        .unwrap_or_default();
    if screens.is_empty() {
        screens = run_output("xdpyinfo", &[])
            .map(|output| parse_xdpyinfo_screens(&output))
            .unwrap_or_default();
    }
    if screens.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "could not discover an X11 screen; ensure DISPLAY is set and xrandr or xdpyinfo is installed",
        ));
    }
    Ok(screens)
}

#[cfg(not(target_os = "linux"))]
fn linux_screens() -> Result<Vec<ScreenInfo>> {
    Err(FerrisError::new(
        ErrorKind::Platform,
        "native Linux X11 capture is only available on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn capture_linux_root(path: &Path) -> Result<()> {
    let display = std::env::var("DISPLAY").unwrap_or_default();
    if display.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "DISPLAY is not set; run FerrisGrid inside an X11 session such as Xvfb/noVNC",
        ));
    }
    let status = Command::new("import")
        .arg("-window")
        .arg("root")
        .arg(path)
        .status()
        .map_err(|error| {
            FerrisError::new(
                ErrorKind::Capture,
                format!("failed to run ImageMagick import: {error}"),
            )
        })?;
    if !status.success() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            "ImageMagick import failed; ensure the X11 display is reachable and imagemagick is installed",
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_output(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output().map_err(|error| {
        FerrisError::new(
            ErrorKind::Capture,
            format!("failed to run {program}: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(FerrisError::new(
            ErrorKind::Capture,
            format!("{program} failed while querying the X11 display"),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(any(target_os = "linux", test))]
fn parse_xrandr_screens(output: &str) -> Vec<ScreenInfo> {
    let mut screens = output
        .lines()
        .filter(|line| line.contains(" connected"))
        .filter_map(parse_xrandr_screen)
        .collect::<Vec<_>>();
    screens.sort_by_key(|screen| {
        (
            !screen.is_primary,
            screen.origin_y,
            screen.origin_x,
            screen.name.clone(),
        )
    });
    for (index, screen) in screens.iter_mut().enumerate() {
        screen.screen_id = format!("screen-{}", index + 1);
        screen.is_primary = index == 0 || screen.is_primary;
    }
    screens
}

#[cfg(any(target_os = "linux", test))]
fn parse_xrandr_screen(line: &str) -> Option<ScreenInfo> {
    let mut fields = line.split_whitespace();
    let name = fields.next()?.to_string();
    if fields.next()? != "connected" {
        return None;
    }
    let is_primary = line.split_whitespace().any(|field| field == "primary");
    let geometry = line
        .split_whitespace()
        .find_map(|field| parse_geometry(field))?;
    Some(ScreenInfo {
        screen_id: String::new(),
        name,
        is_primary,
        origin_x: geometry.2,
        origin_y: geometry.3,
        native_width: geometry.0,
        native_height: geometry.1,
        scale_factor: 1.0,
    })
}

#[cfg(any(target_os = "linux", test))]
fn parse_geometry(value: &str) -> Option<(u32, u32, i32, i32)> {
    let (width, rest) = value.split_once('x')?;
    let x_start = rest.find(['+', '-'])?;
    let height = &rest[..x_start];
    let coords = &rest[x_start..];
    let second_sign = coords[1..].find(['+', '-']).map(|index| index + 1)?;
    let origin_x = &coords[..second_sign];
    let origin_y = &coords[second_sign..];
    Some((
        width.parse().ok()?,
        height.parse().ok()?,
        origin_x.parse().ok()?,
        origin_y.parse().ok()?,
    ))
}

#[cfg(any(target_os = "linux", test))]
fn parse_xdpyinfo_screens(output: &str) -> Vec<ScreenInfo> {
    output
        .lines()
        .find_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("dimensions:")?.trim();
            let dimensions = rest.split_whitespace().next()?;
            let (width, height) = dimensions.split_once('x')?;
            Some(vec![ScreenInfo {
                screen_id: "screen-1".to_string(),
                name: "X11 Screen".to_string(),
                is_primary: true,
                origin_x: 0,
                origin_y: 0,
                native_width: width.parse().ok()?,
                native_height: height.parse().ok()?,
                scale_factor: 1.0,
            }])
        })
        .unwrap_or_default()
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

#[allow(dead_code)]
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
    max_image_edge: Option<u32>,
) -> Result<Vec<CapturedScreen>> {
    fs::create_dir_all(frame_dir)?;
    let mut captured = Vec::new();
    for screen in screens {
        let (image_width, image_height) =
            scaled_dimensions(screen.native_width, screen.native_height, max_image_edge);
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
            "## Screen Metadata\n- screen_id: {}\n- name: {}\n- coordinate_mode: normalized-1000\n- coordinate_range: x=0..1000 y=0..1000\n- coordinate_origin: top_left\n- coordinate_scope: screen_local\n- image_mapping: image_x=round(x/1000*(image_width-1)) image_y=round(y/1000*(image_height-1))\n- native_mapping: native_x=origin_x+round(x/1000*native_width) native_y=origin_y+round(y/1000*native_height)\n- origin_x: {}\n- origin_y: {}\n- native_width: {}\n- native_height: {}\n- image_width: {}\n- image_height: {}\n- scale_factor: {}\n- is_primary: {}\n- screenshot: {}\n",
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

fn downsample_image(path: &Path, max_image_edge: Option<u32>) -> Result<()> {
    let Some(max_edge) = max_image_edge.filter(|edge| *edge > 0) else {
        return Ok(());
    };
    let image = ImageReader::open(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
        .with_guessed_format()
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?
        .decode()
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
    let (width, height) = (image.width(), image.height());
    let longest = width.max(height);
    if longest <= max_edge {
        return Ok(());
    }
    let (new_width, new_height) = scaled_dimensions(width, height, Some(max_edge));
    image
        .resize(new_width, new_height, FilterType::Triangle)
        .save(path)
        .map_err(|error| FerrisError::new(ErrorKind::Capture, error.to_string()))?;
    Ok(())
}

fn scaled_dimensions(width: u32, height: u32, max_image_edge: Option<u32>) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let Some(max_edge) = max_image_edge.filter(|edge| *edge > 0) else {
        return (width, height);
    };
    let longest = width.max(height);
    if longest <= max_edge {
        return (width, height);
    }
    let scale = max_edge as f64 / longest as f64;
    (
        ((width as f64 * scale).round() as u32).max(1),
        ((height as f64 * scale).round() as u32).max(1),
    )
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
    let label = Rgba([255, 255, 255, 245]);
    let label_bg = Rgba([0, 0, 0, 190]);

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

    for tick in (0..=1000).step_by(100) {
        let x = normalized_to_pixel(tick, width);
        let y = normalized_to_pixel(tick, height);
        draw_centered_label(image, x, 8, &tick.to_string(), label, label_bg);
        draw_label(
            image,
            8,
            y.saturating_sub(8),
            &tick.to_string(),
            label,
            label_bg,
        );
    }
    draw_label(image, 8, 8, "0,0", axis, label_bg);
    let center_y = normalized_to_pixel(500, height).saturating_add(10);
    draw_centered_label(
        image,
        normalized_to_pixel(500, width),
        center_y,
        "500,500",
        axis,
        label_bg,
    );
    let bottom_y = height.saturating_sub(22);
    draw_centered_label(
        image,
        normalized_to_pixel(1000, width),
        bottom_y,
        "1000,1000",
        major,
        label_bg,
    );
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

fn draw_centered_label(
    image: &mut RgbaImage,
    center_x: u32,
    y: u32,
    text: &str,
    color: Rgba<u8>,
    background: Rgba<u8>,
) {
    let (image_width, _) = image.dimensions();
    let label_width = text_width(text).saturating_add(4);
    let x = center_x
        .saturating_sub(label_width / 2)
        .min(image_width.saturating_sub(label_width.saturating_add(1)));
    draw_label(image, x, y, text, color, background);
}

fn draw_label(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    text: &str,
    color: Rgba<u8>,
    background: Rgba<u8>,
) {
    let (width, height) = image.dimensions();
    let text_width = text_width(text);
    let bg_width = text_width.saturating_add(4);
    let bg_height = 16;
    let x = x.min(width.saturating_sub(1));
    let y = y.min(height.saturating_sub(1));
    let x_end = x.saturating_add(bg_width).min(width.saturating_sub(1));
    let y_end = y.saturating_add(bg_height).min(height.saturating_sub(1));
    for px in x..=x_end {
        for py in y..=y_end {
            blend_pixel(image, px, py, background);
        }
    }
    let mut cursor = x.saturating_add(2);
    for ch in text.chars() {
        draw_char(image, cursor, y.saturating_add(3), ch, color);
        cursor = cursor.saturating_add(char_width(ch).saturating_add(2));
    }
}

fn text_width(text: &str) -> u32 {
    text.chars()
        .map(|ch| char_width(ch).saturating_add(2))
        .sum::<u32>()
        .saturating_sub(2)
}

fn char_width(ch: char) -> u32 {
    match ch {
        ',' | '.' => 2,
        '-' => 4,
        _ => 8,
    }
}

fn draw_char(image: &mut RgbaImage, x: u32, y: u32, ch: char, color: Rgba<u8>) {
    let Some(pattern) = glyph(ch) else {
        return;
    };
    for (row, bits) in pattern.iter().enumerate() {
        for col in 0..5 {
            if (bits & (1 << (4 - col))) != 0 {
                let px = x.saturating_add((col * 2) as u32);
                let py = y.saturating_add((row * 2) as u32);
                draw_square(image, px, py, color, 1);
            }
        }
    }
}

fn glyph(ch: char) -> Option<[u8; 5]> {
    match ch {
        '0' => Some([0b11111, 0b10001, 0b10001, 0b10001, 0b11111]),
        '1' => Some([0b00100, 0b01100, 0b00100, 0b00100, 0b01110]),
        '2' => Some([0b11111, 0b00001, 0b11111, 0b10000, 0b11111]),
        '3' => Some([0b11111, 0b00001, 0b11111, 0b00001, 0b11111]),
        '4' => Some([0b10001, 0b10001, 0b11111, 0b00001, 0b00001]),
        '5' => Some([0b11111, 0b10000, 0b11111, 0b00001, 0b11111]),
        '6' => Some([0b11111, 0b10000, 0b11111, 0b10001, 0b11111]),
        '7' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b00100]),
        '8' => Some([0b11111, 0b10001, 0b11111, 0b10001, 0b11111]),
        '9' => Some([0b11111, 0b10001, 0b11111, 0b00001, 0b11111]),
        ',' => Some([0b00, 0b00, 0b00, 0b10, 0b10]),
        '-' => Some([0b00000, 0b00000, 0b11110, 0b00000, 0b00000]),
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xrandr_screen_topology() {
        let screens = parse_xrandr_screens(
            "Screen 0: minimum 8 x 8, current 3200 x 1080, maximum 32767 x 32767\n\
             HDMI-1 connected 1920x1080+1280+0 normal left inverted right x axis y axis\n\
             eDP-1 connected primary 1280x800+0+0 normal left inverted right x axis y axis\n",
        );

        assert_eq!(screens.len(), 2);
        assert_eq!(screens[0].screen_id, "screen-1");
        assert_eq!(screens[0].name, "eDP-1");
        assert!(screens[0].is_primary);
        assert_eq!(screens[0].native_width, 1280);
        assert_eq!(screens[0].native_height, 800);
        assert_eq!(screens[1].screen_id, "screen-2");
        assert_eq!(screens[1].origin_x, 1280);
    }

    #[test]
    fn parses_xvfb_xrandr_default_screen() {
        let screens = parse_xrandr_screens(
            "Screen 0: minimum 1 x 1, current 1280 x 800, maximum 8192 x 8192\n\
             screen connected primary 1280x800+0+0 0mm x 0mm\n",
        );

        assert_eq!(screens.len(), 1);
        assert_eq!(screens[0].screen_id, "screen-1");
        assert_eq!(screens[0].native_width, 1280);
        assert_eq!(screens[0].native_height, 800);
    }

    #[test]
    fn parses_xdpyinfo_dimensions_fallback() {
        let screens = parse_xdpyinfo_screens(
            "name of display:    :99\n\
             screen #0:\n\
               dimensions:    1440x900 pixels (381x238 millimeters)\n",
        );

        assert_eq!(screens.len(), 1);
        assert_eq!(screens[0].screen_id, "screen-1");
        assert_eq!(screens[0].native_width, 1440);
        assert_eq!(screens[0].native_height, 900);
    }
}
