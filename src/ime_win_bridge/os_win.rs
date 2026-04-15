//! IMM and window-message helpers for Windows IME state probing.
//!
//! ## Notes
//! - This module intentionally keeps both IMM querying and `WM_IME_*` subclass hooks.
//! - Different IMEs/drivers may expose different subsets of signals.

#[cfg(windows)]
use std::sync::{Mutex, OnceLock};
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::Ime::{
    GCS_COMPSTR, GCS_RESULTSTR, IMN_CLOSECANDIDATE, IMN_OPENCANDIDATE, ImmGetCompositionStringW,
    ImmGetContext, ImmGetDefaultIMEWnd, ImmGetOpenStatus, ImmReleaseContext,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    SendMessageW, SetWindowLongPtrW, GUITHREADINFO, GWLP_WNDPROC, WM_IME_COMPOSITION,
    WM_IME_CONTROL, WM_IME_ENDCOMPOSITION, WM_IME_NOTIFY, WM_IME_STARTCOMPOSITION, WNDPROC,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImeStatus {
    /// IME open status (on/off), suitable for "IME activated" checks.
    pub is_open: bool,
    /// Current composition text byte length from IMM (`GCS_COMPSTR`).
    /// `> 0` means preedit is in progress.
    pub composition_len: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImeMessageSnapshot {
    pub seq: u64,
    pub composing: bool,
    pub candidate_open: bool,
    pub start_count: u64,
    pub end_count: u64,
    pub composition_count: u64,
    pub notify_open_count: u64,
    pub notify_close_count: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionState {
    Active,
    Inactive,
    Unknown,
}

#[allow(dead_code)]
impl ImeStatus {
    pub fn is_composing(self) -> bool {
        self.composition_len > 0
    }

    pub fn composition_state(self) -> CompositionState {
        if self.composition_len > 0 {
            CompositionState::Active
        } else if self.composition_len == 0 {
            CompositionState::Inactive
        } else {
            CompositionState::Unknown
        }
    }
}

#[cfg(windows)]
const IMC_GETOPENSTATUS_MSG: usize = 0x0005;

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct ImeHookState {
    hwnd: isize,
    old_proc: isize,
    installed: bool,
    seq: u64,
    composing: bool,
    start_count: u64,
    end_count: u64,
    composition_count: u64,
    notify_open_count: u64,
    notify_close_count: u64,
    candidate_open: bool,
}

#[cfg(windows)]
impl ImeHookState {
    const fn new() -> Self {
        Self {
            hwnd: 0,
            old_proc: 0,
            installed: false,
            seq: 0,
            composing: false,
            start_count: 0,
            end_count: 0,
            composition_count: 0,
            notify_open_count: 0,
            notify_close_count: 0,
            candidate_open: false,
        }
    }
}

#[cfg(windows)]
fn ime_hook_state() -> &'static Mutex<ImeHookState> {
    static STATE: OnceLock<Mutex<ImeHookState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ImeHookState::new()))
}

#[cfg(windows)]
unsafe extern "system" fn ime_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let old_proc = {
        let mut state = ime_hook_state().lock().expect("ime hook state poisoned");
        match msg {
            WM_IME_STARTCOMPOSITION => {
                state.seq += 1;
                state.composing = true;
                state.start_count += 1;
                log::debug!(
                    "ime-wndmsg START composing={} candidate_open={} start={} end={} comp={} open={} close={} seq={}",
                    state.composing,
                    state.candidate_open,
                    state.start_count,
                    state.end_count,
                    state.composition_count,
                    state.notify_open_count,
                    state.notify_close_count,
                    state.seq
                );
            }
            WM_IME_ENDCOMPOSITION => {
                state.seq += 1;
                state.composing = false;
                state.end_count += 1;
                log::debug!(
                    "ime-wndmsg END composing={} candidate_open={} start={} end={} comp={} open={} close={} seq={}",
                    state.composing,
                    state.candidate_open,
                    state.start_count,
                    state.end_count,
                    state.composition_count,
                    state.notify_open_count,
                    state.notify_close_count,
                    state.seq
                );
            }
            WM_IME_NOTIFY => {
                let cmd = wparam.0 as u32;
                match cmd {
                    IMN_OPENCANDIDATE => {
                        state.seq += 1;
                        state.notify_open_count += 1;
                        state.candidate_open = true;
                        log::debug!(
                            "ime-wndmsg NOTIFY_OPEN composing={} candidate_open={} start={} end={} comp={} open={} close={} seq={}",
                            state.composing,
                            state.candidate_open,
                            state.start_count,
                            state.end_count,
                            state.composition_count,
                            state.notify_open_count,
                            state.notify_close_count,
                            state.seq
                        );
                    }
                    IMN_CLOSECANDIDATE => {
                        state.seq += 1;
                        state.notify_close_count += 1;
                        state.candidate_open = false;
                        log::debug!(
                            "ime-wndmsg NOTIFY_CLOSE composing={} candidate_open={} start={} end={} comp={} open={} close={} seq={}",
                            state.composing,
                            state.candidate_open,
                            state.start_count,
                            state.end_count,
                            state.composition_count,
                            state.notify_open_count,
                            state.notify_close_count,
                            state.seq
                        );
                    }
                    _ => {}
                }
            }
            WM_IME_COMPOSITION => {
                state.seq += 1;
                state.composition_count += 1;
                let flags = lparam.0 as u32;
                let has_comp = (flags & GCS_COMPSTR.0) != 0;
                let has_result = (flags & GCS_RESULTSTR.0) != 0;
                if has_comp {
                    state.composing = true;
                }
                log::debug!(
                    "ime-wndmsg COMPOSITION composing={} candidate_open={} has_comp={} has_result={} flags=0x{:X} start={} end={} comp={} open={} close={} seq={}",
                    state.composing,
                    state.candidate_open,
                    has_comp,
                    has_result,
                    flags,
                    state.start_count,
                    state.end_count,
                    state.composition_count,
                    state.notify_open_count,
                    state.notify_close_count,
                    state.seq
                );
            }
            _ => {}
        }
        state.old_proc
    };

    if old_proc != 0 {
        // SAFETY: old_proc is the original window procedure returned by SetWindowLongPtrW.
        let prev_wnd_proc: WNDPROC = Some(std::mem::transmute(old_proc));
        CallWindowProcW(prev_wnd_proc, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn install_ime_message_hook(hwnd_value: isize) -> bool {
    unsafe {
        if hwnd_value == 0 {
            return false;
        }

        let hwnd = HWND(hwnd_value);
        let mut state = ime_hook_state().lock().expect("ime hook state poisoned");

        if state.installed && state.hwnd == hwnd_value {
            return true;
        }

        if state.installed && state.hwnd != 0 && state.old_proc != 0 {
            let _ = SetWindowLongPtrW(HWND(state.hwnd), GWLP_WNDPROC, state.old_proc);
            state.installed = false;
        }

        let prev = SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            ime_subclass_proc as *const () as usize as isize,
        );
        if prev == 0 {
            return false;
        }

        state.hwnd = hwnd_value;
        state.old_proc = prev;
        state.installed = true;
        state.seq = 0;
        state.composing = false;
        state.start_count = 0;
        state.end_count = 0;
        state.composition_count = 0;
        state.notify_open_count = 0;
        state.notify_close_count = 0;
        state.candidate_open = false;
        true
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn uninstall_ime_message_hook() {
    unsafe {
        let mut state = ime_hook_state().lock().expect("ime hook state poisoned");
        if state.installed && state.hwnd != 0 && state.old_proc != 0 {
            let _ = SetWindowLongPtrW(HWND(state.hwnd), GWLP_WNDPROC, state.old_proc);
        }
        *state = ImeHookState::new();
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn ime_message_snapshot() -> Option<ImeMessageSnapshot> {
    let state = ime_hook_state().lock().ok()?;
    if !state.installed {
        return None;
    }
    Some(ImeMessageSnapshot {
        seq: state.seq,
        composing: state.composing,
        candidate_open: state.candidate_open,
        start_count: state.start_count,
        end_count: state.end_count,
        composition_count: state.composition_count,
        notify_open_count: state.notify_open_count,
        notify_close_count: state.notify_close_count,
    })
}

#[cfg(windows)]
#[allow(dead_code)]
fn focused_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    unsafe {
        let hwnd_foreground = GetForegroundWindow();
        if hwnd_foreground.0 == 0 {
            return None;
        }

        let thread_id = GetWindowThreadProcessId(hwnd_foreground, None);
        if thread_id == 0 {
            return Some(hwnd_foreground);
        }

        let mut info = GUITHREADINFO {
            cbSize: core::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        if GetGUIThreadInfo(thread_id, &mut info).is_ok() && info.hwndFocus.0 != 0 {
            Some(info.hwndFocus)
        } else {
            Some(hwnd_foreground)
        }
    }
}

#[cfg(windows)]
#[allow(dead_code)]
fn query_open_status_via_ime_window(hwnd: windows::Win32::Foundation::HWND) -> Option<bool> {
    unsafe {
        let ime_hwnd = ImmGetDefaultIMEWnd(hwnd);
        if ime_hwnd.0 == 0 {
            return None;
        }
        let ret = SendMessageW(
            ime_hwnd,
            WM_IME_CONTROL,
            WPARAM(IMC_GETOPENSTATUS_MSG),
            LPARAM(0),
        );
        Some(ret.0 != 0)
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn query_ime_status() -> Option<ImeStatus> {
    unsafe {
        let hwnd = focused_hwnd()?;
        let open_from_ime_window = query_open_status_via_ime_window(hwnd);
        let himc = ImmGetContext(hwnd);
        if himc.0 == 0 {
            // 跨线程/跨进程时，ImmGetContext 可能失败；此时尽量返回 IME 开关状态。
            return open_from_ime_window.map(|is_open| ImeStatus {
                is_open,
                composition_len: -1,
            });
        }

        let is_open_from_context = ImmGetOpenStatus(himc).as_bool();
        let is_open = open_from_ime_window.unwrap_or(is_open_from_context);
        let composition_len = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
        let _ = ImmReleaseContext(hwnd, himc);

        Some(ImeStatus {
            is_open,
            composition_len,
        })
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn is_ime_open() -> bool {
    query_ime_status().is_some_and(|s| s.is_open)
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn is_ime_composing() -> bool {
    query_ime_status().is_some_and(ImeStatus::is_composing)
}

#[cfg(not(windows))]
pub fn query_ime_status() -> Option<ImeStatus> {
    None
}

#[cfg(not(windows))]
pub fn is_ime_open() -> bool {
    false
}

#[cfg(not(windows))]
pub fn is_ime_composing() -> bool {
    false
}

#[cfg(not(windows))]
pub fn install_ime_message_hook(_hwnd_value: isize) -> bool {
    false
}

#[cfg(not(windows))]
pub fn uninstall_ime_message_hook() {}

#[cfg(not(windows))]
pub fn ime_message_snapshot() -> Option<ImeMessageSnapshot> {
    None
}
