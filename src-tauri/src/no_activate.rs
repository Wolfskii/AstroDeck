//! Keep the main deck window from taking OS focus.
//!
//! Games on another monitor minimize or lose the cursor if a click activates
//! AstroDeck. The main window still receives clicks and drags. The settings
//! window is left alone so it can take focus for typing.

#[cfg(windows)]
mod platform {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use tauri::Manager;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, EnumChildWindows, GetAncestor, GetForegroundWindow, GetWindowLongPtrW,
        GetWindowPlacement, IsIconic, SetForegroundWindow, SetWindowLongPtrW, SetWindowPlacement,
        ShowWindow, GA_ROOT, GWL_EXSTYLE, GWLP_WNDPROC, SW_SHOWNOACTIVATE, WINDOWPLACEMENT,
        WINEVENT_OUTOFCONTEXT, WNDPROC,
    };

    const WM_MOUSEACTIVATE: u32 = 0x0021;
    const WM_ACTIVATE: u32 = 0x0006;
    const WM_PARENTNOTIFY: u32 = 0x0210;
    const WM_CREATE: u32 = 0x0001;
    const MA_NOACTIVATE: isize = 3;
    const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;

    static MAIN_ROOT: Mutex<isize> = Mutex::new(0);
    static LAST_OTHER: Mutex<isize> = Mutex::new(0);
    static SUBCLASSED: Mutex<Option<HashSet<isize>>> = Mutex::new(None);
    static PREV_PROCS: Mutex<Vec<(isize, isize)>> = Mutex::new(Vec::new());

    pub fn install(window: &tauri::WebviewWindow) {
        let Ok(hwnd) = native_hwnd(window) else {
            log::warn!("Could not read the main window handle for no-activate");
            return;
        };
        unsafe {
            *MAIN_ROOT.lock().expect("main hwnd") = hwnd_key(hwnd);
            apply_tree(hwnd);
            let _ = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_hook),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
        }
        let app = window.app_handle().clone();
        std::thread::spawn(move || {
            for delay_ms in [400, 1200, 3000] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                let app_for_thread = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(window) = app_for_thread.get_webview_window("main") {
                        if let Ok(hwnd) = native_hwnd(&window) {
                            unsafe { apply_tree(hwnd) };
                        }
                    }
                });
            }
        });
        log::info!("Main window will not take focus from other apps");
    }

    pub fn show_without_activating(window: &tauri::WebviewWindow) -> Result<(), String> {
        let hwnd = native_hwnd(window)?;
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let mut placement = WINDOWPLACEMENT::default();
                placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                if GetWindowPlacement(hwnd, &mut placement).is_ok() {
                    placement.showCmd = SW_SHOWNOACTIVATE.0 as u32;
                    let _ = SetWindowPlacement(hwnd, &placement);
                }
            }
            ShowWindow(hwnd, SW_SHOWNOACTIVATE).ok().map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn native_hwnd(window: &tauri::WebviewWindow) -> Result<HWND, String> {
        let foreign = window.hwnd().map_err(|err| err.to_string())?;
        Ok(HWND(foreign.0))
    }

    fn hwnd_key(hwnd: HWND) -> isize {
        hwnd.0 as isize
    }

    unsafe fn apply_tree(root: HWND) {
        subclass_one(root);
        let _ = EnumChildWindows(root, Some(enum_child), LPARAM(0));
    }

    unsafe extern "system" fn enum_child(hwnd: HWND, _: LPARAM) -> BOOL {
        subclass_one(hwnd);
        BOOL(1)
    }

    unsafe fn subclass_one(hwnd: HWND) {
        if hwnd.0.is_null() {
            return;
        }
        let key = hwnd_key(hwnd);
        let mut seen = SUBCLASSED.lock().expect("subclass set");
        let seen = seen.get_or_insert_with(HashSet::new);
        if !seen.insert(key) {
            return;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE);
        let previous = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, deck_wnd_proc as *const () as isize);
        if previous != 0 {
            PREV_PROCS.lock().expect("wndprocs").push((key, previous));
        }
    }

    unsafe extern "system" fn deck_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_MOUSEACTIVATE {
            remember_other_foreground();
            return LRESULT(MA_NOACTIVATE);
        }
        if msg == WM_PARENTNOTIFY && (wparam.0 & 0xFFFF) == WM_CREATE as usize {
            let child = HWND(lparam.0 as *mut std::ffi::c_void);
            subclass_one(child);
        }
        if msg == WM_ACTIVATE && (wparam.0 & 0xFFFF) != 0 {
            restore_previous();
        }
        let previous = previous_proc(hwnd);
        if previous == 0 {
            return LRESULT(0);
        }
        CallWindowProcW(
            WNDPROC::from(std::mem::transmute::<
                isize,
                unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
            >(previous)),
            hwnd,
            msg,
            wparam,
            lparam,
        )
    }

    fn previous_proc(hwnd: HWND) -> isize {
        let key = hwnd_key(hwnd);
        PREV_PROCS
            .lock()
            .expect("wndprocs")
            .iter()
            .find(|(stored, _)| *stored == key)
            .map(|(_, proc)| *proc)
            .unwrap_or(0)
    }

    fn remember_other_foreground() {
        unsafe {
            let current = GetForegroundWindow();
            if current.0.is_null() || belongs_to_main(current) {
                return;
            }
            *LAST_OTHER.lock().expect("foreground") = hwnd_key(current);
        }
    }

    fn restore_previous() {
        let previous = *LAST_OTHER.lock().expect("foreground");
        if previous == 0 {
            return;
        }
        unsafe {
            let hwnd = HWND(previous as *mut std::ffi::c_void);
            if hwnd.0.is_null() || belongs_to_main(hwnd) {
                return;
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }

    fn belongs_to_main(hwnd: HWND) -> bool {
        let main = *MAIN_ROOT.lock().expect("main hwnd");
        if main == 0 || hwnd.0.is_null() {
            return false;
        }
        unsafe {
            let root = GetAncestor(hwnd, GA_ROOT);
            hwnd_key(root) == main || hwnd_key(hwnd) == main
        }
    }

    unsafe extern "system" fn foreground_hook(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        _object: i32,
        _child: i32,
        _thread: u32,
        _time: u32,
    ) {
        if event != EVENT_SYSTEM_FOREGROUND || hwnd.0.is_null() {
            return;
        }
        if belongs_to_main(hwnd) {
            restore_previous();
            return;
        }
        *LAST_OTHER.lock().expect("foreground") = hwnd_key(hwnd);
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn install(_window: &tauri::WebviewWindow) {}

    pub fn show_without_activating(window: &tauri::WebviewWindow) -> Result<(), String> {
        window.show().map_err(|err| err.to_string())?;
        window.unminimize().map_err(|err| err.to_string())?;
        Ok(())
    }
}

pub fn install(window: &tauri::WebviewWindow) {
    platform::install(window);
}

pub fn show_without_activating(window: &tauri::WebviewWindow) -> Result<(), String> {
    platform::show_without_activating(window)
}
