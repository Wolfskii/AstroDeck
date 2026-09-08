/// Handle Teams-related actions.
pub fn handle(command: &str) -> Result<(), String> {
    match command {
        "toggleMute" => send_teams_shortcut("Ctrl+Shift+M", 0x4D, true),
        "toggleCamera" => send_teams_shortcut("Ctrl+Shift+O", 0x4F, true),
        "shareScreen" => send_teams_shortcut("Ctrl+Shift+E", 0x45, true),
        "raiseHand" => send_teams_shortcut("Ctrl+Shift+K", 0x4B, true),
        "chat" => send_teams_shortcut("Ctrl+Shift+R", 0x52, true),
        "leaveMeeting" => send_teams_shortcut("Ctrl+Shift+H", 0x48, true),
        "goCalendar" => send_teams_shortcut("Ctrl+4", 0x34, false),
        "goActivity" => send_teams_shortcut("Ctrl+1", 0x31, false),
        cmd if cmd.starts_with("reaction.") => Err(
            "Teams reactions require UI automation; Teams exposes no reaction keyboard shortcut."
                .to_string(),
        ),
        _ => {
            log::warn!("Unknown Teams command: {}", command);
            Err(format!("Unknown Teams command: {}", command))
        }
    }
}

#[cfg(windows)]
fn send_teams_shortcut(label: &str, key: u8, shift: bool) -> Result<(), String> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, keybd_event,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible, SW_RESTORE,
        SetForegroundWindow, ShowWindow,
    };

    unsafe extern "system" fn find_teams_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return BOOL(1);
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        if written <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buffer[..written as usize]).to_lowercase();
        if title.contains("microsoft teams") || title.contains("teams") {
            let target = unsafe { &mut *(lparam.0 as *mut Option<HWND>) };
            *target = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    let mut target = None;
    unsafe {
        let _ = EnumWindows(
            Some(find_teams_window),
            LPARAM(&mut target as *mut Option<HWND> as isize),
        );
    }
    let Some(hwnd) = target else {
        return Err("Could not find an open Microsoft Teams meeting window.".to_string());
    };

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
        keybd_event(0x11, 0, KEYBD_EVENT_FLAGS(0), 0);
        if shift {
            keybd_event(0x10, 0, KEYBD_EVENT_FLAGS(0), 0);
        }
        keybd_event(key, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(key, 0, KEYEVENTF_KEYUP, 0);
        if shift {
            keybd_event(0x10, 0, KEYEVENTF_KEYUP, 0);
        }
        keybd_event(0x11, 0, KEYEVENTF_KEYUP, 0);
    }
    log::info!("Teams: sent {label}");
    Ok(())
}

#[cfg(not(windows))]
fn send_teams_shortcut(label: &str, _key: u8, _shift: bool) -> Result<(), String> {
    Err(format!("Teams shortcut {label} is only supported on Windows"))
}
