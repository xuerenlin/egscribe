//! TSF (Text Services Framework) monitoring helpers.
//!
//! Background:
//! - On some environments, `ITfContextOwnerCompositionSink` is not connectable
//!   (`CONNECT_E_CANNOTCONNECT`).
//! - In that case we fallback to `ITfUIElementSink` lifecycle (`UI_BEGIN/UI_END`) as
//!   an approximation of "IME preedit UI is currently active".

#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
use std::cell::RefCell;
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use windows::core::{implement, Interface, IUnknown, Result as WinResult};
#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, RPC_E_CHANGED_MODE};
#[cfg(windows)]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
#[cfg(windows)]
use windows::Win32::UI::TextServices::{
    CLSID_TF_ThreadMgr, ITfCompositionView, ITfContext, ITfContextOwnerCompositionSink,
    ITfContextOwnerCompositionSink_Impl, ITfDocumentMgr, ITfRange, ITfSource, ITfThreadMgr, ITfUIElementSink,
    ITfUIElementSink_Impl, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl, TF_INVALID_COOKIE,
};

#[cfg(windows)]
const CONNECT_E_CANNOTCONNECT_HRESULT: i32 = 0x8004_0202u32 as i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TsfSnapshot {
    pub seq: u64,
    pub composing: bool,
    pub start_count: u64,
    pub update_count: u64,
    pub end_count: u64,
    pub context_bound: bool,
    pub ui_open: bool,
    pub ui_begin_count: u64,
    pub ui_update_count: u64,
    pub ui_end_count: u64,
    pub last_ui_end_ms: u64,
    pub composition_sink_supported: bool,
}

#[cfg(windows)]
#[derive(Default)]
struct TsfRuntime {
    installed: bool,
    com_initialized_here: bool,
    thread_mgr: Option<ITfThreadMgr>,
    thread_source: Option<ITfSource>,
    thread_cookie: u32,
    thread_sink: Option<ITfThreadMgrEventSink>,
    ui_cookie: u32,
    ui_sink: Option<ITfUIElementSink>,
    context_source: Option<ITfSource>,
    context_cookie: u32,
    context_sink: Option<ITfContextOwnerCompositionSink>,
    snapshot: TsfSnapshot,
    bind_retry_count: u64,
    composition_sink_blocked: bool,
}

#[cfg(windows)]
thread_local! {
    static TSF_RUNTIME: RefCell<TsfRuntime> = RefCell::new(TsfRuntime {
        thread_cookie: TF_INVALID_COOKIE,
        ui_cookie: TF_INVALID_COOKIE,
        context_cookie: TF_INVALID_COOKIE,
        bind_retry_count: 0,
        ..Default::default()
    });
}

#[cfg(windows)]
fn now_ms() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs() * 1000 + u64::from(now.subsec_millis())
}

#[cfg(windows)]
#[implement(ITfThreadMgrEventSink)]
struct ThreadMgrSink;

#[cfg(windows)]
impl ITfThreadMgrEventSink_Impl for ThreadMgrSink {
    fn OnInitDocumentMgr(&self, _pdim: Option<&ITfDocumentMgr>) -> WinResult<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _pdim: Option<&ITfDocumentMgr>) -> WinResult<()> {
        Ok(())
    }

    fn OnSetFocus(
        &self,
        pdimfocus: Option<&ITfDocumentMgr>,
        _pdimprevfocus: Option<&ITfDocumentMgr>,
    ) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            let focus = pdimfocus.cloned();
            unsafe {
                let _ = bind_context_sink_locked(&mut rt, focus);
            }
        });
        Ok(())
    }

    fn OnPushContext(&self, _pic: Option<&ITfContext>) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            unsafe {
                let _ = bind_context_sink_locked(&mut rt, None);
            }
        });
        Ok(())
    }

    fn OnPopContext(&self, _pic: Option<&ITfContext>) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            unsafe {
                let _ = bind_context_sink_locked(&mut rt, None);
            }
        });
        Ok(())
    }
}

#[cfg(windows)]
#[implement(ITfContextOwnerCompositionSink)]
struct CompositionSink;

#[cfg(windows)]
impl ITfContextOwnerCompositionSink_Impl for CompositionSink {
    fn OnStartComposition(&self, _pcomposition: Option<&ITfCompositionView>) -> WinResult<BOOL> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.composing = true;
            rt.snapshot.start_count += 1;
            log::debug!(
                "tsf START composing={} start={} update={} end={} seq={} bound={}",
                rt.snapshot.composing,
                rt.snapshot.start_count,
                rt.snapshot.update_count,
                rt.snapshot.end_count,
                rt.snapshot.seq,
                rt.snapshot.context_bound
            );
        });
        Ok(BOOL(1))
    }

    fn OnUpdateComposition(
        &self,
        _pcomposition: Option<&ITfCompositionView>,
        _prangenew: Option<&ITfRange>,
    ) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.composing = true;
            rt.snapshot.update_count += 1;
            log::debug!(
                "tsf UPDATE composing={} start={} update={} end={} seq={} bound={}",
                rt.snapshot.composing,
                rt.snapshot.start_count,
                rt.snapshot.update_count,
                rt.snapshot.end_count,
                rt.snapshot.seq,
                rt.snapshot.context_bound
            );
        });
        Ok(())
    }

    fn OnEndComposition(&self, _pcomposition: Option<&ITfCompositionView>) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.composing = false;
            rt.snapshot.end_count += 1;
            log::debug!(
                "tsf END composing={} start={} update={} end={} seq={} bound={}",
                rt.snapshot.composing,
                rt.snapshot.start_count,
                rt.snapshot.update_count,
                rt.snapshot.end_count,
                rt.snapshot.seq,
                rt.snapshot.context_bound
            );
        });
        Ok(())
    }
}

#[cfg(windows)]
#[implement(ITfUIElementSink)]
struct UiElementSink;

#[cfg(windows)]
impl ITfUIElementSink_Impl for UiElementSink {
    fn BeginUIElement(&self, dwuielementid: u32, pbshow: *mut BOOL) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.ui_open = true;
            rt.snapshot.ui_begin_count += 1;
            log::debug!(
                "tsf UI_BEGIN id={} show_ptr_null={} composing={} begin={} update={} end={} seq={}",
                dwuielementid,
                pbshow.is_null(),
                rt.snapshot.composing,
                rt.snapshot.ui_begin_count,
                rt.snapshot.ui_update_count,
                rt.snapshot.ui_end_count,
                rt.snapshot.seq
            );
        });
        Ok(())
    }

    fn UpdateUIElement(&self, dwuielementid: u32) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.ui_update_count += 1;
            log::debug!(
                "tsf UI_UPDATE id={} composing={} begin={} update={} end={} seq={}",
                dwuielementid,
                rt.snapshot.composing,
                rt.snapshot.ui_begin_count,
                rt.snapshot.ui_update_count,
                rt.snapshot.ui_end_count,
                rt.snapshot.seq
            );
        });
        Ok(())
    }

    fn EndUIElement(&self, dwuielementid: u32) -> WinResult<()> {
        TSF_RUNTIME.with(|cell| {
            let mut rt = cell.borrow_mut();
            rt.snapshot.seq += 1;
            rt.snapshot.ui_open = false;
            rt.snapshot.ui_end_count += 1;
            rt.snapshot.last_ui_end_ms = now_ms();
            log::debug!(
                "tsf UI_END id={} composing={} begin={} update={} end={} seq={}",
                dwuielementid,
                rt.snapshot.composing,
                rt.snapshot.ui_begin_count,
                rt.snapshot.ui_update_count,
                rt.snapshot.ui_end_count,
                rt.snapshot.seq
            );
        });
        Ok(())
    }
}

#[cfg(windows)]
unsafe fn advise_sink<T: Interface>(source: &ITfSource, sink: &T) -> WinResult<u32> {
    let punk: IUnknown = sink.cast()?;
    source.AdviseSink(&T::IID, &punk)
}

#[cfg(windows)]
unsafe fn unbind_context_sink_locked(rt: &mut TsfRuntime) {
    if let (Some(src), cookie) = (&rt.context_source, rt.context_cookie) {
        if cookie != TF_INVALID_COOKIE {
            let _ = src.UnadviseSink(cookie);
        }
    }
    rt.context_source = None;
    rt.context_cookie = TF_INVALID_COOKIE;
    rt.snapshot.context_bound = false;
}

#[cfg(windows)]
unsafe fn bind_context_sink_locked(
    rt: &mut TsfRuntime,
    focus_doc_mgr: Option<ITfDocumentMgr>,
) -> WinResult<()> {
    if rt.composition_sink_blocked {
        return Ok(());
    }

    unbind_context_sink_locked(rt);
    let Some(ctx_sink) = rt.context_sink.clone() else {
        log::warn!("tsf bind skipped: no context sink");
        return Ok(());
    };

    let doc_mgr = if let Some(dm) = focus_doc_mgr {
        dm
    } else if let Some(tm) = &rt.thread_mgr {
        match tm.GetFocus() {
            Ok(dm) => dm,
            Err(e) => {
                log::debug!("tsf bind skipped: ITfThreadMgr::GetFocus failed: {e:?}");
                return Ok(());
            }
        }
    } else {
        log::warn!("tsf bind skipped: no thread manager");
        return Ok(());
    };

    let context = match doc_mgr.GetTop() {
        Ok(c) => c,
        Err(e) => {
            log::debug!("tsf bind skipped: ITfDocumentMgr::GetTop failed: {e:?}");
            return Ok(());
        }
    };

    let source: ITfSource = match context.cast() {
        Ok(s) => s,
        Err(e) => {
            log::debug!("tsf bind skipped: context cast ITfSource failed: {e:?}");
            return Ok(());
        }
    };

    let cookie = match advise_sink(&source, &ctx_sink) {
        Ok(cookie) => cookie,
        Err(e) => {
            if e.code().0 == CONNECT_E_CANNOTCONNECT_HRESULT {
                rt.composition_sink_blocked = true;
                rt.snapshot.composition_sink_supported = false;
                log::warn!(
                    "tsf bind unsupported: ITfContextOwnerCompositionSink not connectable (CONNECT_E_CANNOTCONNECT)"
                );
                return Ok(());
            }
            log::warn!("tsf bind failed: AdviseSink(ITfContextOwnerCompositionSink) err={e:?}");
            return Err(e);
        }
    };
    rt.context_source = Some(source);
    rt.context_cookie = cookie;
    rt.snapshot.context_bound = true;
    rt.bind_retry_count = 0;
    rt.snapshot.composition_sink_supported = true;
    log::debug!("tsf context sink bound: cookie={}", cookie);
    Ok(())
}

#[cfg(windows)]
pub fn install_tsf_monitor() -> bool {
    TSF_RUNTIME.with(|cell| {
        let mut rt = cell.borrow_mut();
        if rt.installed {
            return true;
        }

        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_ok() {
                rt.com_initialized_here = true;
            } else if hr != RPC_E_CHANGED_MODE {
                log::warn!("tsf install failed: CoInitializeEx hr={:?}", hr);
                return false;
            }

            let thread_mgr: ITfThreadMgr =
                match CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("tsf install failed: CoCreateInstance ITfThreadMgr err={e:?}");
                        return false;
                    }
                };

            if let Err(e) = thread_mgr.Activate() {
                log::warn!("tsf install failed: ITfThreadMgr::Activate err={e:?}");
                return false;
            }

            let thread_source: ITfSource = match thread_mgr.cast() {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("tsf install failed: cast ITfSource err={e:?}");
                    let _ = thread_mgr.Deactivate();
                    return false;
                }
            };

            let thread_sink_obj: ITfThreadMgrEventSink = ThreadMgrSink.into();
            let ui_sink_obj: ITfUIElementSink = UiElementSink.into();
            let context_sink_obj: ITfContextOwnerCompositionSink = CompositionSink.into();

            let thread_cookie = match advise_sink(&thread_source, &thread_sink_obj) {
                Ok(cookie) => cookie,
                Err(e) => {
                    log::warn!("tsf install failed: AdviseSink thread err={e:?}");
                    let _ = thread_mgr.Deactivate();
                    return false;
                }
            };
            let ui_cookie = match advise_sink(&thread_source, &ui_sink_obj) {
                Ok(cookie) => cookie,
                Err(e) => {
                    log::warn!("tsf install warning: AdviseSink UIElement err={e:?}");
                    TF_INVALID_COOKIE
                }
            };

            rt.thread_mgr = Some(thread_mgr);
            rt.thread_source = Some(thread_source);
            rt.thread_cookie = thread_cookie;
            rt.thread_sink = Some(thread_sink_obj);
            rt.ui_cookie = ui_cookie;
            rt.ui_sink = Some(ui_sink_obj);
            rt.context_sink = Some(context_sink_obj);
            rt.snapshot = TsfSnapshot {
                seq: 0,
                composing: false,
                start_count: 0,
                update_count: 0,
                end_count: 0,
                context_bound: false,
                ui_open: false,
                ui_begin_count: 0,
                ui_update_count: 0,
                ui_end_count: 0,
                last_ui_end_ms: 0,
                composition_sink_supported: true,
            };
            rt.composition_sink_blocked = false;

            let _ = bind_context_sink_locked(&mut rt, None);
            rt.installed = true;
            log::info!(
                "tsf monitor installed: context_bound={} ui_sink={}",
                rt.snapshot.context_bound,
                rt.ui_cookie != TF_INVALID_COOKIE
            );
            true
        }
    })
}

#[cfg(windows)]
pub fn poll_tsf_snapshot() -> Option<TsfSnapshot> {
    TSF_RUNTIME.with(|cell| {
        let rt = cell.borrow();
        if rt.installed {
            Some(rt.snapshot)
        } else {
            None
        }
    })
}

#[cfg(windows)]
pub fn should_suppress_backspace_after_ui_end(window_ms: u64) -> bool {
    TSF_RUNTIME.with(|cell| {
        let rt = cell.borrow();
        if !rt.installed {
            return false;
        }
        if rt.snapshot.composition_sink_supported {
            return false;
        }
        if rt.snapshot.ui_open || rt.snapshot.last_ui_end_ms == 0 {
            return false;
        }
        now_ms().saturating_sub(rt.snapshot.last_ui_end_ms) <= window_ms
    })
}

#[cfg(windows)]
pub fn ensure_tsf_context_bound() {
    TSF_RUNTIME.with(|cell| unsafe {
        let mut rt = cell.borrow_mut();
        if !rt.installed || rt.snapshot.context_bound {
            return;
        }
        if rt.composition_sink_blocked {
            return;
        }
        rt.bind_retry_count += 1;
        if rt.bind_retry_count % 25 == 1 {
            log::debug!("tsf rebinding attempt #{}", rt.bind_retry_count);
        }
        let _ = bind_context_sink_locked(&mut rt, None);
    });
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn uninstall_tsf_monitor() {
    TSF_RUNTIME.with(|cell| unsafe {
        let mut rt = cell.borrow_mut();
        if !rt.installed {
            return;
        }

        unbind_context_sink_locked(&mut rt);
        if let (Some(src), cookie) = (&rt.thread_source, rt.thread_cookie) {
            if cookie != TF_INVALID_COOKIE {
                let _ = src.UnadviseSink(cookie);
            }
            if rt.ui_cookie != TF_INVALID_COOKIE {
                let _ = src.UnadviseSink(rt.ui_cookie);
            }
        }
        if let Some(tm) = &rt.thread_mgr {
            let _ = tm.Deactivate();
        }
        if rt.com_initialized_here {
            CoUninitialize();
        }

        rt.installed = false;
        rt.com_initialized_here = false;
        rt.thread_mgr = None;
        rt.thread_source = None;
        rt.thread_cookie = TF_INVALID_COOKIE;
        rt.thread_sink = None;
        rt.ui_sink = None;
        rt.ui_cookie = TF_INVALID_COOKIE;
        rt.context_sink = None;
        rt.snapshot.context_bound = false;
    });
}

#[cfg(not(windows))]
pub fn install_tsf_monitor() -> bool {
    false
}

#[cfg(not(windows))]
pub fn poll_tsf_snapshot() -> Option<TsfSnapshot> {
    None
}

#[cfg(not(windows))]
pub fn should_suppress_backspace_after_ui_end(_window_ms: u64) -> bool {
    false
}

#[cfg(not(windows))]
pub fn ensure_tsf_context_bound() {}

#[cfg(not(windows))]
pub fn uninstall_tsf_monitor() {}
