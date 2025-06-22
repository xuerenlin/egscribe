
/* 
use windows::{
    Win32::UI::Input::Ime::{
        ImmGetContext, ImmGetCompositionStringW, ImmReleaseContext, GCS_COMPSTR, ImmGetConversionStatus, IME_CONVERSION_MODE, IME_SENTENCE_MODE
    },
    Win32::UI::WindowsAndMessaging::GetForegroundWindow,
};

pub fn is_ime_active() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        let himc = ImmGetContext(hwnd);
        
        log::debug!("himc is {:?}", himc);

        // 检查组合字符串长度
        let len = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
        log::debug!("len is {:?}", len);

        let mut conversion = IME_CONVERSION_MODE(0);
        let mut sentence = IME_SENTENCE_MODE(0);
        ImmGetConversionStatus(himc, Some(&mut conversion), Some(&mut sentence));
        log::debug!("conversion is {:?}", himc);
        log::debug!("sentence is {:?}", sentence);

        ImmReleaseContext(hwnd, himc);

        len > 0
    }
}
*/

//pub fn is_ime_active() -> bool {
//    false
//}
