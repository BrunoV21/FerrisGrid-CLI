use crate::recorder::{EventSource, EventSourceCapabilities};
use crate::reducer::RawInputEvent;
#[cfg(target_os = "macos")]
use crate::reducer::{ControlEvent, Modifiers};
#[cfg(target_os = "macos")]
use ferrisgrid_core::MouseButton;
use ferrisgrid_core::{ErrorKind, FerrisError, Result};
#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
#[cfg(target_os = "macos")]
use std::sync::atomic::Ordering;
use std::sync::mpsc::SyncSender;
#[cfg(target_os = "macos")]
use std::sync::mpsc::TrySendError;

#[derive(Debug, Clone)]
pub struct RecordingPermissionReport {
    pub supported: bool,
    pub screen_capture: bool,
    pub input_observation: bool,
    pub accessibility: bool,
    pub detail: String,
}

pub struct MacOsEventSource;

impl MacOsEventSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSource for MacOsEventSource {
    fn name(&self) -> &'static str {
        "native-macos-event-tap"
    }

    fn capabilities(&self) -> EventSourceCapabilities {
        EventSourceCapabilities {
            mouse: cfg!(target_os = "macos"),
            keyboard: cfg!(target_os = "macos"),
            global_controls: cfg!(target_os = "macos"),
        }
    }

    fn run(&self, sender: SyncSender<RawInputEvent>, stop: Arc<AtomicBool>) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            run_macos_event_tap(sender, stop)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (sender, stop);
            Err(FerrisError::new(
                ErrorKind::Platform,
                "native demonstration recording is currently available on macOS only; use --backend fake for protocol tests",
            ))
        }
    }
}

pub fn recording_permission_report() -> RecordingPermissionReport {
    #[cfg(target_os = "macos")]
    unsafe {
        let screen_capture = CGPreflightScreenCaptureAccess();
        let input_observation = CGPreflightListenEventAccess();
        let accessibility = AXIsProcessTrusted();
        let supported = macos_major_version().is_some_and(|version| version >= 13);
        RecordingPermissionReport {
            supported,
            screen_capture,
            input_observation,
            accessibility,
            detail: if !supported {
                "native demonstration recording requires macOS 13 or newer".to_string()
            } else if screen_capture && input_observation && accessibility {
                "ready".to_string()
            } else {
                "grant Screen Recording, Input Monitoring, and Accessibility access to the terminal or FerrisGrid binary"
                    .to_string()
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        RecordingPermissionReport {
            supported: false,
            screen_capture: false,
            input_observation: false,
            accessibility: false,
            detail: "native recorder not implemented for this operating system".to_string(),
        }
    }
}

pub fn native_event_source() -> Box<dyn EventSource> {
    Box::new(MacOsEventSource::new())
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
struct EventTapContext {
    sender: SyncSender<RawInputEvent>,
    stop: Arc<AtomicBool>,
    paused: AtomicBool,
    overflow: AtomicBool,
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn event_tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: *mut c_void,
    user_info: *mut c_void,
) -> *mut c_void {
    if event.is_null() || user_info.is_null() {
        return event;
    }
    let context = unsafe { &*(user_info as *const EventTapContext) };
    if context.stop.load(Ordering::Relaxed) {
        return event;
    }
    if event_type == EVENT_KEY_DOWN {
        let flags = unsafe { CGEventGetFlags(event) };
        let key_code = unsafe { CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16 };
        let modifiers = modifiers_from_flags(flags);
        let control_chord = modifiers.control && modifiers.option && modifiers.command;
        if control_chord && key_code == KEY_ESCAPE {
            context.stop.store(true, Ordering::Relaxed);
            send_event(
                context,
                RawInputEvent::Control {
                    at_ms: unix_millis(),
                    control: ControlEvent::Stop,
                },
            );
            return std::ptr::null_mut();
        }
        if control_chord && key_code == KEY_P {
            let was_paused = context.paused.fetch_xor(true, Ordering::Relaxed);
            send_event(
                context,
                RawInputEvent::Control {
                    at_ms: unix_millis(),
                    control: if was_paused {
                        ControlEvent::Resume
                    } else {
                        ControlEvent::Pause
                    },
                },
            );
            return std::ptr::null_mut();
        }
    }
    if context.paused.load(Ordering::Relaxed) {
        return event;
    }
    let at_ms = unix_millis();
    let raw = match event_type {
        EVENT_LEFT_MOUSE_DOWN | EVENT_RIGHT_MOUSE_DOWN | EVENT_OTHER_MOUSE_DOWN => {
            let point = unsafe { CGEventGetLocation(event) };
            Some(RawInputEvent::MouseDown {
                at_ms,
                x: point.x.round() as i32,
                y: point.y.round() as i32,
                button: mouse_button(event_type),
                click_count: unsafe {
                    CGEventGetIntegerValueField(event, FIELD_MOUSE_CLICK_STATE)
                        .clamp(1, u8::MAX as i64) as u8
                },
            })
        }
        EVENT_LEFT_MOUSE_UP | EVENT_RIGHT_MOUSE_UP | EVENT_OTHER_MOUSE_UP => {
            let point = unsafe { CGEventGetLocation(event) };
            Some(RawInputEvent::MouseUp {
                at_ms,
                x: point.x.round() as i32,
                y: point.y.round() as i32,
                button: mouse_button(event_type),
                click_count: unsafe {
                    CGEventGetIntegerValueField(event, FIELD_MOUSE_CLICK_STATE)
                        .clamp(1, u8::MAX as i64) as u8
                },
            })
        }
        EVENT_LEFT_MOUSE_DRAGGED | EVENT_RIGHT_MOUSE_DRAGGED | EVENT_OTHER_MOUSE_DRAGGED => {
            let point = unsafe { CGEventGetLocation(event) };
            Some(RawInputEvent::MouseMove {
                at_ms,
                x: point.x.round() as i32,
                y: point.y.round() as i32,
            })
        }
        EVENT_SCROLL_WHEEL => {
            let point = unsafe { CGEventGetLocation(event) };
            Some(RawInputEvent::Scroll {
                at_ms,
                x: point.x.round() as i32,
                y: point.y.round() as i32,
                delta_x: unsafe {
                    CGEventGetIntegerValueField(event, FIELD_SCROLL_DELTA_AXIS_2) as i32
                },
                delta_y: unsafe {
                    CGEventGetIntegerValueField(event, FIELD_SCROLL_DELTA_AXIS_1) as i32
                },
            })
        }
        EVENT_KEY_DOWN => {
            let flags = unsafe { CGEventGetFlags(event) };
            let key_code =
                unsafe { CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16 };
            let text = unsafe { keyboard_unicode(event) };
            let key = key_name(key_code, text.as_deref());
            Some(RawInputEvent::KeyDown {
                at_ms,
                key,
                text,
                modifiers: modifiers_from_flags(flags),
                repeat: unsafe {
                    CGEventGetIntegerValueField(event, FIELD_KEYBOARD_AUTOREPEAT) != 0
                },
            })
        }
        _ => None,
    };
    if let Some(raw) = raw {
        send_event(context, raw);
    }
    event
}

#[cfg(target_os = "macos")]
fn run_macos_event_tap(sender: SyncSender<RawInputEvent>, stop: Arc<AtomicBool>) -> Result<()> {
    if macos_major_version().is_none_or(|version| version < 13) {
        return Err(FerrisError::new(
            ErrorKind::Platform,
            "ferrisgrid record requires macOS 13 or newer",
        ));
    }
    let report = recording_permission_report();
    if !report.input_observation || !report.accessibility {
        return Err(FerrisError::new(ErrorKind::Permission, report.detail));
    }
    let context = Box::new(EventTapContext {
        sender,
        stop: stop.clone(),
        paused: AtomicBool::new(false),
        overflow: AtomicBool::new(false),
    });
    let context_ptr = Box::into_raw(context);
    let mask = event_mask(&[
        EVENT_LEFT_MOUSE_DOWN,
        EVENT_LEFT_MOUSE_UP,
        EVENT_RIGHT_MOUSE_DOWN,
        EVENT_RIGHT_MOUSE_UP,
        EVENT_LEFT_MOUSE_DRAGGED,
        EVENT_RIGHT_MOUSE_DRAGGED,
        EVENT_OTHER_MOUSE_DOWN,
        EVENT_OTHER_MOUSE_UP,
        EVENT_OTHER_MOUSE_DRAGGED,
        EVENT_SCROLL_WHEEL,
        EVENT_KEY_DOWN,
    ]);
    let tap = unsafe {
        CGEventTapCreate(
            SESSION_EVENT_TAP,
            HEAD_INSERT_EVENT_TAP,
            ACTIVE_EVENT_TAP,
            mask,
            Some(event_tap_callback),
            context_ptr.cast(),
        )
    };
    if tap.is_null() {
        unsafe { drop(Box::from_raw(context_ptr)) };
        return Err(FerrisError::new(
            ErrorKind::Permission,
            "could not create macOS event tap; grant Input Monitoring and Accessibility permission",
        ));
    }
    let source = unsafe { CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0) };
    if source.is_null() {
        unsafe {
            CFRelease(tap);
            drop(Box::from_raw(context_ptr));
        }
        return Err(FerrisError::new(
            ErrorKind::Platform,
            "failed to create the macOS event-tap run-loop source",
        ));
    }
    let run_loop = unsafe { CFRunLoopGetCurrent() };
    unsafe {
        CFRunLoopAddSource(run_loop, source, kCFRunLoopDefaultMode);
        CGEventTapEnable(tap, true);
    }
    while !stop.load(Ordering::Relaxed) {
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, true);
        }
    }
    let overflow = unsafe { (*context_ptr).overflow.load(Ordering::Relaxed) };
    unsafe {
        CFRunLoopRemoveSource(run_loop, source, kCFRunLoopDefaultMode);
        CFRelease(source);
        CFRelease(tap);
        drop(Box::from_raw(context_ptr));
    }
    if overflow {
        Err(FerrisError::new(
            ErrorKind::Execution,
            "recording event queue overflowed; the session was stopped to avoid silently losing input",
        ))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn send_event(context: &EventTapContext, event: RawInputEvent) {
    match context.sender.try_send(event) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
            context.overflow.store(true, Ordering::Relaxed);
            context.stop.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(target_os = "macos")]
fn event_mask(events: &[u32]) -> u64 {
    events
        .iter()
        .fold(0_u64, |mask, event| mask | (1_u64 << event))
}

#[cfg(target_os = "macos")]
fn mouse_button(event_type: u32) -> MouseButton {
    match event_type {
        EVENT_RIGHT_MOUSE_DOWN | EVENT_RIGHT_MOUSE_UP | EVENT_RIGHT_MOUSE_DRAGGED => {
            MouseButton::Right
        }
        EVENT_OTHER_MOUSE_DOWN | EVENT_OTHER_MOUSE_UP | EVENT_OTHER_MOUSE_DRAGGED => {
            MouseButton::Middle
        }
        _ => MouseButton::Left,
    }
}

#[cfg(target_os = "macos")]
fn modifiers_from_flags(flags: u64) -> Modifiers {
    Modifiers {
        command: flags & FLAG_COMMAND != 0,
        control: flags & FLAG_CONTROL != 0,
        option: flags & FLAG_OPTION != 0,
        shift: flags & FLAG_SHIFT != 0,
    }
}

#[cfg(target_os = "macos")]
unsafe fn keyboard_unicode(event: *mut c_void) -> Option<String> {
    let mut actual = 0_usize;
    let mut buffer = [0_u16; 32];
    unsafe {
        CGEventKeyboardGetUnicodeString(event, buffer.len(), &mut actual, buffer.as_mut_ptr());
    }
    if actual == 0 {
        return None;
    }
    let value = String::from_utf16_lossy(&buffer[..actual.min(buffer.len())]);
    (!value.chars().all(char::is_control)).then_some(value)
}

#[cfg(target_os = "macos")]
fn key_name(key_code: u16, text: Option<&str>) -> String {
    match key_code {
        36 | 76 => "enter",
        48 => "tab",
        49 => "space",
        51 => "backspace",
        53 => "escape",
        115 => "home",
        116 => "pageup",
        117 => "delete",
        119 => "end",
        121 => "pagedown",
        123 => "left",
        124 => "right",
        125 => "down",
        126 => "up",
        122 => "f1",
        120 => "f2",
        99 => "f3",
        118 => "f4",
        96 => "f5",
        97 => "f6",
        98 => "f7",
        100 => "f8",
        101 => "f9",
        109 => "f10",
        103 => "f11",
        111 => "f12",
        _ => {
            return text
                .unwrap_or(&format!("keycode-{key_code}"))
                .to_lowercase();
        }
    }
    .to_string()
}

#[cfg(target_os = "macos")]
fn macos_major_version() -> Option<u32> {
    let output = std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .split('.')
        .next()?
        .parse()
        .ok()
}

#[cfg(target_os = "macos")]
fn unix_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(target_os = "macos")]
const EVENT_LEFT_MOUSE_DOWN: u32 = 1;
#[cfg(target_os = "macos")]
const EVENT_LEFT_MOUSE_UP: u32 = 2;
#[cfg(target_os = "macos")]
const EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
#[cfg(target_os = "macos")]
const EVENT_RIGHT_MOUSE_UP: u32 = 4;
#[cfg(target_os = "macos")]
const EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
#[cfg(target_os = "macos")]
const EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
#[cfg(target_os = "macos")]
const EVENT_KEY_DOWN: u32 = 10;
#[cfg(target_os = "macos")]
const EVENT_SCROLL_WHEEL: u32 = 22;
#[cfg(target_os = "macos")]
const EVENT_OTHER_MOUSE_DOWN: u32 = 25;
#[cfg(target_os = "macos")]
const EVENT_OTHER_MOUSE_UP: u32 = 26;
#[cfg(target_os = "macos")]
const EVENT_OTHER_MOUSE_DRAGGED: u32 = 27;
#[cfg(target_os = "macos")]
const FIELD_MOUSE_CLICK_STATE: u32 = 1;
#[cfg(target_os = "macos")]
const FIELD_KEYBOARD_AUTOREPEAT: u32 = 8;
#[cfg(target_os = "macos")]
const FIELD_KEYBOARD_KEYCODE: u32 = 9;
#[cfg(target_os = "macos")]
const FIELD_SCROLL_DELTA_AXIS_1: u32 = 11;
#[cfg(target_os = "macos")]
const FIELD_SCROLL_DELTA_AXIS_2: u32 = 12;
#[cfg(target_os = "macos")]
const FLAG_SHIFT: u64 = 1 << 17;
#[cfg(target_os = "macos")]
const FLAG_CONTROL: u64 = 1 << 18;
#[cfg(target_os = "macos")]
const FLAG_OPTION: u64 = 1 << 19;
#[cfg(target_os = "macos")]
const FLAG_COMMAND: u64 = 1 << 20;
#[cfg(target_os = "macos")]
const KEY_P: u16 = 35;
#[cfg(target_os = "macos")]
const KEY_ESCAPE: u16 = 53;
#[cfg(target_os = "macos")]
const SESSION_EVENT_TAP: u32 = 1;
#[cfg(target_os = "macos")]
const HEAD_INSERT_EVENT_TAP: u32 = 0;
#[cfg(target_os = "macos")]
const ACTIVE_EVENT_TAP: u32 = 0;

#[cfg(target_os = "macos")]
type EventTapCallback =
    unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: Option<EventTapCallback>,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CGEventGetFlags(event: *mut c_void) -> u64;
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventKeyboardGetUnicodeString(
        event: *mut c_void,
        max_length: usize,
        actual_length: *mut usize,
        unicode_string: *mut u16,
    );
    fn CGPreflightListenEventAccess() -> bool;
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: *const c_void;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRemoveSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: bool) -> i32;
    fn CFRelease(value: *mut c_void);
}
