use crypto::digest::Digest;
use crypto::md5::Md5;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn cache_dir(work_dir: &Path) -> PathBuf {
    work_dir
        .parent()
        .map(|p| p.join("cache"))
        .unwrap_or_else(|| PathBuf::from("./cache"))
}

pub fn ensure_cache_dir(work_dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(cache_dir(work_dir))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn path_md5(path: &str) -> String {
    let normalized = normalize_path(path);
    let mut hasher = Md5::new();
    hasher.input(normalized.as_bytes());
    hasher.result_str()
}

pub fn is_untitled_path(path: &str) -> bool {
    path.starts_with("untitled/")
}

/// Returns whether this file path should use swap files.
pub fn should_use_swap(_source_path: &str) -> bool {
    true
}

pub fn swap_path(source_path: &str, work_dir: &Path) -> PathBuf {
    let path = Path::new(source_path);
    let md5 = path_md5(source_path);

    let name_prefix = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .or_else(|| path.file_name().and_then(|s| s.to_str()))
        .unwrap_or("file");

    let ext = path.extension().and_then(|e| e.to_str());

    let file_name = if let Some(ext) = ext {
        format!("{}_{}.{}", name_prefix, md5, ext)
    } else {
        format!("{}_{}", name_prefix, md5)
    };

    cache_dir(work_dir).join(file_name)
}

fn file_modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

pub fn write_swap(source_path: &str, work_dir: &Path, text: &str) -> io::Result<()> {
    if !should_use_swap(source_path) {
        return Ok(());
    }
    ensure_cache_dir(work_dir)?;
    let swap = swap_path(source_path, work_dir);
    std::fs::write(&swap, text)
}

pub fn delete_swap(source_path: &str, work_dir: &Path) -> io::Result<()> {
    if !should_use_swap(source_path) {
        return Ok(());
    }
    let swap = swap_path(source_path, work_dir);
    if swap.exists() {
        std::fs::remove_file(&swap)?;
    }
    Ok(())
}

fn resolve_untitled_on_open(
    source_path: &str,
    work_dir: &Path,
) -> io::Result<(String, bool)> {
    ensure_cache_dir(work_dir)?;
    let swap = swap_path(source_path, work_dir);

    if swap.exists() {
        match std::fs::read_to_string(&swap) {
            Ok(content) => {
                log::info!(
                    "Recovered unsaved edits from swap for {}",
                    source_path
                );
                return Ok((content, true));
            }
            Err(e) => {
                log::warn!(
                    "Failed to read swap file {}, falling back to empty: {}",
                    swap.display(),
                    e
                );
            }
        }
    }

    write_swap(source_path, work_dir, "")?;
    Ok(("".to_string(), false))
}

/// Resolve which content to load when opening a file.
/// Returns (content, loaded_from_swap).
pub fn resolve_content_on_open(
    source_path: &str,
    work_dir: &Path,
    source_text: &str,
) -> io::Result<(String, bool)> {
    if !should_use_swap(source_path) {
        return Ok((source_text.to_string(), false));
    }

    if is_untitled_path(source_path) {
        return resolve_untitled_on_open(source_path, work_dir);
    }

    ensure_cache_dir(work_dir)?;
    let swap = swap_path(source_path, work_dir);
    let source = Path::new(source_path);

    if !swap.exists() {
        write_swap(source_path, work_dir, source_text)?;
        return Ok((source_text.to_string(), false));
    }

    let swap_mtime = file_modified_time(&swap);
    let source_mtime = file_modified_time(source);

    let use_swap = match (swap_mtime, source_mtime) {
        (Some(sw), Some(so)) => sw > so,
        (Some(_), None) => true,
        _ => false,
    };

    if use_swap {
        match std::fs::read_to_string(&swap) {
            Ok(content) => {
                log::info!(
                    "Recovered unsaved edits from swap for {}",
                    source_path
                );
                return Ok((content, true));
            }
            Err(e) => {
                log::warn!(
                    "Failed to read swap file {}, falling back to source: {}",
                    swap.display(),
                    e
                );
            }
        }
    }

    write_swap(source_path, work_dir, source_text)?;
    Ok((source_text.to_string(), false))
}
