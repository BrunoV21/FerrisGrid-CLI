#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use ferrisgrid_core::MouseButton;
use ferrisgrid_core::{
    ErrorKind, FerrisError, InputBackend, InputCapabilities, InputExecution, NativeAction, Result,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use std::thread;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use std::time::Duration;

pub struct FakeInputBackend;

impl InputBackend for FakeInputBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn capabilities(&self) -> InputCapabilities {
        InputCapabilities {
            can_mouse: true,
            can_keyboard: true,
        }
    }

    fn execute(&self, action: &NativeAction) -> Result<InputExecution> {
        Ok(InputExecution {
            summary: format!("fake_execute {action:?}"),
        })
    }
}

pub struct MacOsInputBackend;

impl InputBackend for MacOsInputBackend {
    fn name(&self) -> &'static str {
        "native-macos"
    }

    fn capabilities(&self) -> InputCapabilities {
        InputCapabilities {
            can_mouse: cfg!(target_os = "macos"),
            can_keyboard: cfg!(target_os = "macos"),
        }
    }

    fn execute(&self, action: &NativeAction) -> Result<InputExecution> {
        #[cfg(target_os = "macos")]
        {
            execute_macos(action)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = action;
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native input is currently implemented for macOS only; use --backend fake",
            ))
        }
    }
}

pub struct LinuxInputBackend;

impl InputBackend for LinuxInputBackend {
    fn name(&self) -> &'static str {
        "native-linux-x11"
    }

    fn capabilities(&self) -> InputCapabilities {
        InputCapabilities {
            can_mouse: cfg!(target_os = "linux"),
            can_keyboard: cfg!(target_os = "linux"),
        }
    }

    fn execute(&self, action: &NativeAction) -> Result<InputExecution> {
        #[cfg(target_os = "linux")]
        {
            execute_linux(action)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = action;
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native Linux X11 input is only available on Linux; use --backend native on this OS or --backend fake",
            ))
        }
    }
}

pub struct WindowsInputBackend;

impl InputBackend for WindowsInputBackend {
    fn name(&self) -> &'static str {
        "native-windows"
    }

    fn capabilities(&self) -> InputCapabilities {
        InputCapabilities {
            can_mouse: cfg!(target_os = "windows"),
            can_keyboard: cfg!(target_os = "windows"),
        }
    }

    fn execute(&self, action: &NativeAction) -> Result<InputExecution> {
        #[cfg(target_os = "windows")]
        {
            execute_windows(action)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = action;
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native Windows input is only available on Windows; use --backend native on this OS or --backend fake",
            ))
        }
    }
}

pub fn backend_by_name(name: &str) -> Box<dyn InputBackend> {
    match name {
        "fake" => Box::new(FakeInputBackend),
        "native" => native_backend(),
        "macos" | "native-macos" => Box::new(MacOsInputBackend),
        "linux" | "x11" | "native-linux" | "native-linux-x11" => Box::new(LinuxInputBackend),
        "windows" | "win32" | "native-windows" => Box::new(WindowsInputBackend),
        _ => native_backend(),
    }
}

fn native_backend() -> Box<dyn InputBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxInputBackend)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsInputBackend)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsInputBackend)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(MacOsInputBackend)
    }
}

#[cfg(target_os = "windows")]
fn execute_windows(action: &NativeAction) -> Result<InputExecution> {
    match action {
        NativeAction::Click { x, y, button } => {
            windows_click(*x, *y, *button, 1)?;
            Ok(InputExecution {
                summary: format!("click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::DoubleClick { x, y, button } => {
            windows_click(*x, *y, *button, 2)?;
            Ok(InputExecution {
                summary: format!("double_click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::RightClick { x, y } => {
            windows_click(*x, *y, MouseButton::Right, 1)?;
            Ok(InputExecution {
                summary: format!("right_click x={x} y={y}"),
            })
        }
        NativeAction::MoveMouse { x, y } => {
            windows_move(*x, *y)?;
            Ok(InputExecution {
                summary: format!("move_mouse x={x} y={y}"),
            })
        }
        NativeAction::Wait { duration_ms } => {
            thread::sleep(Duration::from_millis(*duration_ms));
            Ok(InputExecution {
                summary: format!("wait duration_ms={duration_ms}"),
            })
        }
        NativeAction::Type { text } => {
            windows_type(text)?;
            Ok(InputExecution {
                summary: "type text=<redacted>".to_string(),
            })
        }
        NativeAction::PressKey { key } => {
            windows_press_key(key)?;
            Ok(InputExecution {
                summary: format!("press_key key={key}"),
            })
        }
        NativeAction::Hotkey { keys } => {
            windows_hotkey(keys)?;
            Ok(InputExecution {
                summary: format!("hotkey keys={}", keys.join("+")),
            })
        }
        NativeAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button,
        } => {
            windows_drag(*from_x, *from_y, *to_x, *to_y, *duration_ms, *button)?;
            Ok(InputExecution {
                summary: format!(
                    "drag from_x={from_x} from_y={from_y} to_x={to_x} to_y={to_y} duration_ms={duration_ms} button={}",
                    button.as_str()
                ),
            })
        }
        NativeAction::Scroll {
            x,
            y,
            delta_x,
            delta_y,
        } => {
            if let (Some(x), Some(y)) = (x, y) {
                windows_move(*x, *y)?;
            }
            if *delta_y != 0 {
                send_windows_mouse(MOUSEEVENTF_WHEEL, *delta_y as u32)?;
            }
            if *delta_x != 0 {
                send_windows_mouse(MOUSEEVENTF_HWHEEL, *delta_x as u32)?;
            }
            Ok(InputExecution {
                summary: format!("scroll delta_x={delta_x} delta_y={delta_y}"),
            })
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_move(x: i32, y: i32) -> Result<()> {
    if unsafe { SetCursorPos(x, y) } == 0 {
        return Err(windows_input_error("SetCursorPos failed"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_click(x: i32, y: i32, button: MouseButton, count: u8) -> Result<()> {
    windows_move(x, y)?;
    let (down, up) = windows_button_flags(button);
    for _ in 0..count {
        send_windows_mouse(down, 0)?;
        send_windows_mouse(up, 0)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_drag(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: u64,
    button: MouseButton,
) -> Result<()> {
    windows_move(from_x, from_y)?;
    let (down, up) = windows_button_flags(button);
    send_windows_mouse(down, 0)?;
    let steps = 10_u64;
    let sleep_ms = duration_ms.checked_div(steps).unwrap_or(0);
    for step in 1..=steps {
        let ratio = step as f64 / steps as f64;
        let x = from_x + ((to_x - from_x) as f64 * ratio).round() as i32;
        let y = from_y + ((to_y - from_y) as f64 * ratio).round() as i32;
        windows_move(x, y)?;
        if sleep_ms > 0 {
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    }
    send_windows_mouse(up, 0)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_button_flags(button: MouseButton) -> (u32, u32) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    }
}

#[cfg(target_os = "windows")]
fn send_windows_mouse(flags: u32, data: u32) -> Result<()> {
    let input = WinInput {
        input_type: INPUT_MOUSE,
        value: WinInputValue {
            mouse: MouseInput {
                dx: 0,
                dy: 0,
                mouse_data: data,
                flags,
                time: 0,
                extra_info: 0,
            },
        },
    };
    send_windows_inputs(&[input])
}

#[cfg(target_os = "windows")]
fn windows_type(text: &str) -> Result<()> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for code_unit in text.encode_utf16() {
        inputs.push(windows_keyboard_input(0, code_unit, KEYEVENTF_UNICODE));
        inputs.push(windows_keyboard_input(
            0,
            code_unit,
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
        ));
    }
    send_windows_inputs(&inputs)
}

#[cfg(target_os = "windows")]
fn windows_press_key(key: &str) -> Result<()> {
    let virtual_key = windows_virtual_key(key)?;
    send_windows_inputs(&[
        windows_keyboard_input(virtual_key, 0, 0),
        windows_keyboard_input(virtual_key, 0, KEYEVENTF_KEYUP),
    ])
}

#[cfg(target_os = "windows")]
fn windows_hotkey(keys: &[String]) -> Result<()> {
    if keys.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "hotkey keys are required",
        ));
    }
    let mapped = keys
        .iter()
        .map(|key| windows_virtual_key(key))
        .collect::<Result<Vec<_>>>()?;
    let mut inputs = Vec::with_capacity(mapped.len() * 2);
    for key in &mapped {
        inputs.push(windows_keyboard_input(*key, 0, 0));
    }
    for key in mapped.iter().rev() {
        inputs.push(windows_keyboard_input(*key, 0, KEYEVENTF_KEYUP));
    }
    send_windows_inputs(&inputs)
}

#[cfg(target_os = "windows")]
fn windows_virtual_key(key: &str) -> Result<u16> {
    if let Some(key) = named_windows_virtual_key(key) {
        return Ok(key);
    }
    let mut chars = key.chars();
    let Some(value) = chars.next() else {
        return Err(unsupported_windows_key(key));
    };
    if chars.next().is_some() || value.len_utf16() != 1 {
        return Err(unsupported_windows_key(key));
    }
    let mapped = unsafe { VkKeyScanW(value as u16) };
    if mapped == -1 {
        Err(unsupported_windows_key(key))
    } else {
        Ok((mapped as u16) & 0xff)
    }
}

#[cfg(any(target_os = "windows", test))]
fn named_windows_virtual_key(key: &str) -> Option<u16> {
    match key.to_ascii_lowercase().as_str() {
        "cmd" | "command" | "meta" | "super" | "win" | "windows" => Some(0x5b),
        "ctrl" | "control" => Some(0x11),
        "alt" | "option" => Some(0x12),
        "shift" => Some(0x10),
        "enter" | "return" => Some(0x0d),
        "tab" => Some(0x09),
        "escape" | "esc" => Some(0x1b),
        "space" => Some(0x20),
        "delete" | "del" => Some(0x2e),
        "backspace" => Some(0x08),
        "up" | "arrowup" => Some(0x26),
        "down" | "arrowdown" => Some(0x28),
        "left" | "arrowleft" => Some(0x25),
        "right" | "arrowright" => Some(0x27),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn unsupported_windows_key(key: &str) -> FerrisError {
    FerrisError::new(
        ErrorKind::Protocol,
        format!("unsupported key for native Windows backend: {key}"),
    )
}

#[cfg(target_os = "windows")]
fn windows_keyboard_input(virtual_key: u16, scan: u16, flags: u32) -> WinInput {
    WinInput {
        input_type: INPUT_KEYBOARD,
        value: WinInputValue {
            keyboard: KeyboardInput {
                virtual_key,
                scan,
                flags,
                time: 0,
                extra_info: 0,
            },
        },
    }
}

#[cfg(target_os = "windows")]
fn send_windows_inputs(inputs: &[WinInput]) -> Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<WinInput>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(windows_input_error(
            "SendInput failed; Windows can block input to an elevated application or secure desktop",
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_input_error(context: &str) -> FerrisError {
    FerrisError::new(
        ErrorKind::Execution,
        format!("{context}: {}", std::io::Error::last_os_error()),
    )
}

#[cfg(target_os = "linux")]
fn execute_linux(action: &NativeAction) -> Result<InputExecution> {
    match action {
        NativeAction::Click { x, y, button } => {
            run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            run_xdotool(&["click", xdotool_button(*button)])?;
            Ok(InputExecution {
                summary: format!("click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::DoubleClick { x, y, button } => {
            run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            run_xdotool(&["click", "--repeat", "2", xdotool_button(*button)])?;
            Ok(InputExecution {
                summary: format!("double_click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::RightClick { x, y } => {
            run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            run_xdotool(&["click", "3"])?;
            Ok(InputExecution {
                summary: format!("right_click x={x} y={y}"),
            })
        }
        NativeAction::MoveMouse { x, y } => {
            run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            Ok(InputExecution {
                summary: format!("move_mouse x={x} y={y}"),
            })
        }
        NativeAction::Wait { duration_ms } => {
            thread::sleep(Duration::from_millis(*duration_ms));
            Ok(InputExecution {
                summary: format!("wait duration_ms={duration_ms}"),
            })
        }
        NativeAction::Type { text } => {
            run_xdotool(&["type", "--clearmodifiers", "--", text])?;
            Ok(InputExecution {
                summary: "type text=<redacted>".to_string(),
            })
        }
        NativeAction::PressKey { key } => {
            let mapped = linux_key(key)?;
            run_xdotool(&["key", "--clearmodifiers", &mapped])?;
            Ok(InputExecution {
                summary: format!("press_key key={key}"),
            })
        }
        NativeAction::Hotkey { keys } => {
            let mapped = keys
                .iter()
                .map(|key| linux_key(key))
                .collect::<Result<Vec<_>>>()?;
            let sequence = mapped.join("+");
            run_xdotool(&["key", "--clearmodifiers", &sequence])?;
            Ok(InputExecution {
                summary: format!("hotkey keys={}", keys.join("+")),
            })
        }
        NativeAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button,
        } => {
            let button = xdotool_button(*button);
            run_xdotool(&["mousemove", &from_x.to_string(), &from_y.to_string()])?;
            run_xdotool(&["mousedown", button])?;
            let steps = 10_u64;
            let sleep_ms = duration_ms.checked_div(steps).unwrap_or(0);
            for step in 1..=steps {
                let ratio = step as f64 / steps as f64;
                let x = from_x + ((*to_x - *from_x) as f64 * ratio).round() as i32;
                let y = from_y + ((*to_y - *from_y) as f64 * ratio).round() as i32;
                run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
                if sleep_ms > 0 {
                    thread::sleep(Duration::from_millis(sleep_ms));
                }
            }
            run_xdotool(&["mouseup", button])?;
            Ok(InputExecution {
                summary: format!(
                    "drag from_x={from_x} from_y={from_y} to_x={to_x} to_y={to_y} duration_ms={duration_ms} button={}",
                    button
                ),
            })
        }
        NativeAction::Scroll {
            x,
            y,
            delta_x,
            delta_y,
        } => {
            if let (Some(x), Some(y)) = (x, y) {
                run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            }
            click_scroll(*delta_y, "4", "5")?;
            click_scroll(*delta_x, "6", "7")?;
            Ok(InputExecution {
                summary: format!("scroll delta_x={delta_x} delta_y={delta_y}"),
            })
        }
    }
}

#[cfg(target_os = "linux")]
fn run_xdotool(args: &[&str]) -> Result<()> {
    if std::env::var("DISPLAY").unwrap_or_default().is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Execution,
            "DISPLAY is not set; run FerrisGrid inside an X11 session such as Xvfb/noVNC",
        ));
    }
    let status = Command::new("xdotool")
        .args(args)
        .status()
        .map_err(|error| {
            FerrisError::new(
                ErrorKind::Execution,
                format!("failed to run xdotool: {error}"),
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(FerrisError::new(
            ErrorKind::Execution,
            "xdotool failed while sending input to the X11 display",
        ))
    }
}

#[cfg(target_os = "linux")]
fn xdotool_button(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "1",
        MouseButton::Middle => "2",
        MouseButton::Right => "3",
    }
}

#[cfg(target_os = "linux")]
fn click_scroll(
    delta: i32,
    positive_button: &'static str,
    negative_button: &'static str,
) -> Result<()> {
    let button = if delta > 0 {
        positive_button
    } else if delta < 0 {
        negative_button
    } else {
        return Ok(());
    };
    let clicks = ((delta.unsigned_abs() + 119) / 120).clamp(1, 30);
    for _ in 0..clicks {
        run_xdotool(&["click", button])?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_key(key: &str) -> Result<String> {
    let mapped = match key.to_ascii_lowercase().as_str() {
        "cmd" | "command" | "meta" | "super" => "Super".to_string(),
        "ctrl" | "control" => "ctrl".to_string(),
        "alt" | "option" => "alt".to_string(),
        "shift" => "shift".to_string(),
        "enter" | "return" => "Return".to_string(),
        "tab" => "Tab".to_string(),
        "escape" | "esc" => "Escape".to_string(),
        "space" => "space".to_string(),
        "delete" | "del" => "Delete".to_string(),
        "backspace" => "BackSpace".to_string(),
        "up" | "arrowup" => "Up".to_string(),
        "down" | "arrowdown" => "Down".to_string(),
        "left" | "arrowleft" => "Left".to_string(),
        "right" | "arrowright" => "Right".to_string(),
        value if value.len() == 1 => value.to_string(),
        other => {
            return Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("unsupported key for native Linux X11 backend: {other}"),
            ));
        }
    };
    Ok(mapped)
}

#[cfg(target_os = "macos")]
fn execute_macos(action: &NativeAction) -> Result<InputExecution> {
    match action {
        NativeAction::Click { x, y, button } => {
            mouse_click(*x, *y, *button, 1)?;
            Ok(InputExecution {
                summary: format!("click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::DoubleClick { x, y, button } => {
            mouse_click(*x, *y, *button, 2)?;
            Ok(InputExecution {
                summary: format!("double_click x={x} y={y} button={}", button.as_str()),
            })
        }
        NativeAction::RightClick { x, y } => {
            mouse_click(*x, *y, MouseButton::Right, 1)?;
            Ok(InputExecution {
                summary: format!("right_click x={x} y={y}"),
            })
        }
        NativeAction::MoveMouse { x, y } => {
            mouse_move(*x, *y)?;
            Ok(InputExecution {
                summary: format!("move_mouse x={x} y={y}"),
            })
        }
        NativeAction::Wait { duration_ms } => {
            thread::sleep(Duration::from_millis(*duration_ms));
            Ok(InputExecution {
                summary: format!("wait duration_ms={duration_ms}"),
            })
        }
        NativeAction::Type { text } => {
            run_osascript(&format!(
                "tell application \"System Events\" to keystroke \"{}\"",
                escape_applescript(text)
            ))?;
            Ok(InputExecution {
                summary: "type text=<redacted>".to_string(),
            })
        }
        NativeAction::PressKey { key } => {
            run_osascript(&format!(
                "tell application \"System Events\" to key code {}",
                key_code(key)?
            ))?;
            Ok(InputExecution {
                summary: format!("press_key key={key}"),
            })
        }
        NativeAction::Hotkey { keys } => {
            run_hotkey(keys)?;
            Ok(InputExecution {
                summary: format!("hotkey keys={}", keys.join("+")),
            })
        }
        NativeAction::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
            button,
        } => {
            mouse_drag(*from_x, *from_y, *to_x, *to_y, *duration_ms, *button)?;
            Ok(InputExecution {
                summary: format!(
                    "drag from_x={from_x} from_y={from_y} to_x={to_x} to_y={to_y} duration_ms={duration_ms} button={}",
                    button.as_str()
                ),
            })
        }
        NativeAction::Scroll {
            delta_x, delta_y, ..
        } => {
            scroll(*delta_x, *delta_y)?;
            Ok(InputExecution {
                summary: format!("scroll delta_x={delta_x} delta_y={delta_y}"),
            })
        }
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateMouseEvent(
        source: *const std::ffi::c_void,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> *mut std::ffi::c_void;
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
    fn CGEventCreateScrollWheelEvent(
        source: *const std::ffi::c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        ...
    ) -> *mut std::ffi::c_void;
    fn CFRelease(cf: *mut std::ffi::c_void);
}

#[cfg(target_os = "macos")]
fn mouse_move(x: i32, y: i32) -> Result<()> {
    post_mouse(5, x, y, 0)
}

#[cfg(target_os = "macos")]
fn mouse_click(x: i32, y: i32, button: MouseButton, count: u8) -> Result<()> {
    let (down, up, button_code) = match button {
        MouseButton::Left => (1, 2, 0),
        MouseButton::Right => (3, 4, 1),
        MouseButton::Middle => (25, 26, 2),
    };
    for _ in 0..count {
        post_mouse(down, x, y, button_code)?;
        post_mouse(up, x, y, button_code)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn mouse_drag(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: u64,
    button: MouseButton,
) -> Result<()> {
    let (down, up, dragged, button_code) = match button {
        MouseButton::Left => (1, 2, 6, 0),
        MouseButton::Right => (3, 4, 7, 1),
        MouseButton::Middle => (25, 26, 27, 2),
    };
    post_mouse(down, from_x, from_y, button_code)?;
    let steps = 10_u64;
    let sleep_ms = duration_ms.checked_div(steps).unwrap_or(0);
    for step in 1..=steps {
        let ratio = step as f64 / steps as f64;
        let x = from_x + ((to_x - from_x) as f64 * ratio).round() as i32;
        let y = from_y + ((to_y - from_y) as f64 * ratio).round() as i32;
        post_mouse(dragged, x, y, button_code)?;
        if sleep_ms > 0 {
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    }
    post_mouse(up, to_x, to_y, button_code)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn scroll(delta_x: i32, delta_y: i32) -> Result<()> {
    unsafe {
        let event = CGEventCreateScrollWheelEvent(std::ptr::null(), 0, 2, delta_y, delta_x);
        if event.is_null() {
            return Err(FerrisError::new(
                ErrorKind::Execution,
                "failed to create macOS scroll event; check Accessibility permission",
            ));
        }
        CGEventPost(0, event);
        CFRelease(event);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn post_mouse(event_type: u32, x: i32, y: i32, button: u32) -> Result<()> {
    unsafe {
        let event = CGEventCreateMouseEvent(
            std::ptr::null(),
            event_type,
            CGPoint {
                x: x as f64,
                y: y as f64,
            },
            button,
        );
        if event.is_null() {
            return Err(FerrisError::new(
                ErrorKind::Execution,
                "failed to create macOS mouse event; check Accessibility permission",
            ));
        }
        CGEventPost(0, event);
        CFRelease(event);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<()> {
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| FerrisError::new(ErrorKind::Execution, error.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(FerrisError::new(
            ErrorKind::Execution,
            "osascript failed; check Accessibility permission",
        ))
    }
}

#[cfg(target_os = "macos")]
fn run_hotkey(keys: &[String]) -> Result<()> {
    let Some(last) = keys.last() else {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "hotkey keys are required",
        ));
    };
    let modifiers: Vec<&str> = keys[..keys.len().saturating_sub(1)]
        .iter()
        .filter_map(|key| match key.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "meta" => Some("command down"),
            "ctrl" | "control" => Some("control down"),
            "alt" | "option" => Some("option down"),
            "shift" => Some("shift down"),
            _ => None,
        })
        .collect();
    let command = match key_code(last) {
        Ok(code) => format!("key code {code}"),
        Err(_) => format!("keystroke \"{}\"", escape_applescript(last)),
    };
    let script = if modifiers.is_empty() {
        format!("tell application \"System Events\" to {command}")
    } else {
        format!(
            "tell application \"System Events\" to {command} using {{{}}}",
            modifiers.join(", ")
        )
    };
    run_osascript(&script)
}

#[cfg(target_os = "macos")]
fn key_code(key: &str) -> Result<u16> {
    match key.to_ascii_lowercase().as_str() {
        "enter" | "return" => Ok(36),
        "tab" => Ok(48),
        "escape" | "esc" => Ok(53),
        "space" => Ok(49),
        "backspace" => Ok(51),
        "delete" | "del" => Ok(117),
        "left" | "arrowleft" => Ok(123),
        "right" | "arrowright" => Ok(124),
        "down" | "arrowdown" => Ok(125),
        "up" | "arrowup" => Ok(126),
        "home" => Ok(115),
        "end" => Ok(119),
        "pageup" => Ok(116),
        "pagedown" => Ok(121),
        "f1" => Ok(122),
        "f2" => Ok(120),
        "f3" => Ok(99),
        "f4" => Ok(118),
        "f5" => Ok(96),
        "f6" => Ok(97),
        "f7" => Ok(98),
        "f8" => Ok(100),
        "f9" => Ok(101),
        "f10" => Ok(109),
        "f11" => Ok(103),
        "f12" => Ok(111),
        other => Err(FerrisError::new(
            ErrorKind::Protocol,
            format!("unsupported key for native macOS backend: {other}"),
        )),
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "windows")]
const INPUT_MOUSE: u32 = 0;
#[cfg(target_os = "windows")]
const INPUT_KEYBOARD: u32 = 1;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_WHEEL: u32 = 0x0800;
#[cfg(target_os = "windows")]
const MOUSEEVENTF_HWHEEL: u32 = 0x1000;
#[cfg(target_os = "windows")]
const KEYEVENTF_KEYUP: u32 = 0x0002;
#[cfg(target_os = "windows")]
const KEYEVENTF_UNICODE: u32 = 0x0004;

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct KeyboardInput {
    virtual_key: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
union WinInputValue {
    mouse: MouseInput,
    keyboard: KeyboardInput,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct WinInput {
    input_type: u32,
    value: WinInputValue,
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn SendInput(count: u32, inputs: *const WinInput, input_size: i32) -> u32;
    fn VkKeyScanW(character: u16) -> i16;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_key_aliases_cover_cross_platform_action_names() {
        assert_eq!(named_windows_virtual_key("cmd"), Some(0x5b));
        assert_eq!(named_windows_virtual_key("control"), Some(0x11));
        assert_eq!(named_windows_virtual_key("escape"), Some(0x1b));
        assert_eq!(named_windows_virtual_key("arrowleft"), Some(0x25));
        assert_eq!(named_windows_virtual_key("backspace"), Some(0x08));
    }

    #[test]
    fn windows_backend_aliases_are_explicit() {
        assert_eq!(backend_by_name("windows").name(), "native-windows");
        assert_eq!(backend_by_name("win32").name(), "native-windows");
        assert_eq!(backend_by_name("native-windows").name(), "native-windows");
    }
}
