pub mod encypt;
pub mod localsocket;
pub mod encoding;
pub mod swap_file;
pub mod url;

#[cfg(windows)]
pub mod win_exec;

pub use encypt::{enc_content, dec_content};
pub use localsocket::start_process;
pub use swap_file::{
    cache_dir, delete_swap, is_untitled_path, resolve_content_on_open, write_swap,
};
pub use url::open_url;

use std::path::Path;

/// 系统「另存为」对话框，返回选中路径；用户取消则返回 `None`。
///
/// `initial_dir` 通过 [`rfd::FileDialog::set_directory`] 指定打开时的初始文件夹；
/// 仅当路径存在且为目录时生效。Linux 部分桌面环境下门户对话框可能忽略该设置。
///
/// Windows 上 `rfd` 使用 `SHCreateItemFromParsingName` 设置文件夹；对混合斜杠等未规范化路径可能静默失败，
/// 故在传入 [`rfd::FileDialog`] 前尽量 [`std::fs::canonicalize`]，以提高初始目录是否生效的概率。
pub fn show_save_file_dialog(file_name: &str, initial_dir: Option<&Path>) -> Option<String> {
    use rfd::FileDialog;

    let mut dialog = FileDialog::new()
        .set_title("Save File")
        .add_filter("All Files", &["*"])
        .add_filter("Excel", &["xlsx"])
        .add_filter("Word", &["docx"])
        .add_filter("Text Files", &["txt"])
        .add_filter("Markdown", &["md"])
        .add_filter("Rust", &["rs"])
        .add_filter("C/C++", &["c", "cpp", "h", "hpp"])
        .add_filter("Python", &["py"])
        .add_filter("JavaScript", &["js", "ts"])
        .add_filter("Java", &["java"]);

    let resolved_dir = initial_dir.and_then(|p| {
        if !p.is_dir() {
            return None;
        }
        match std::fs::canonicalize(p) {
            Ok(abs) => Some(abs),
            Err(_) => Some(p.to_path_buf()),
        }
    });

    if let Some(dir) = resolved_dir.as_ref() {
        dialog = dialog.set_directory(dir);
    }

    let file_path = dialog.set_file_name(file_name).save_file();

    file_path.map(|p| p.to_string_lossy().to_string())
}
