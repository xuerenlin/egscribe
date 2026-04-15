//! Windows：无控制台子进程 / 不依赖 `where.exe` 的 PATH 查找，避免 GUI 应用闪黑窗。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// 与 `CreateProcess` 的 `CREATE_NO_WINDOW` 一致，子进程不创建控制台窗口。
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 在 `PATH` 中查找可执行文件（含 `PATHEXT`），语义接近 `where`，但不启动子进程。
pub fn find_executable_in_path(executable: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let has_extension = Path::new(executable).extension().is_some();

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(executable);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !has_extension {
            let pathexts = std::env::var_os("PATHEXT").unwrap_or_else(|| {
                OsString::from(".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC")
            });
            for ext in pathexts.to_string_lossy().split(';') {
                let ext = ext.trim();
                if ext.is_empty() {
                    continue;
                }
                let candidate = dir.join(format!("{}{}", executable, ext));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
