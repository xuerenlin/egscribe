use egscribe::util::swap_file::{
    delete_swap, is_untitled_path, resolve_content_on_open, should_use_swap, write_swap,
};
use std::fs;
use std::path::PathBuf;

fn test_work_dir(test_name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("egscribe_swap_test_{test_name}"));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("note")).unwrap();
    base.join("note")
}

#[test]
fn should_use_swap_includes_untitled_paths() {
    assert!(should_use_swap("untitled/Untitled-1"));
    assert!(should_use_swap("/path/to/file.rs"));
}

#[test]
fn is_untitled_path_detects_virtual_paths() {
    assert!(is_untitled_path("untitled/Untitled-1"));
    assert!(!is_untitled_path("note/hello.md"));
}

#[test]
fn untitled_resolve_without_swap_returns_empty_content() {
    let work_dir = test_work_dir("untitled_no_swap");
    let path = "untitled/Untitled-1";

    let (content, loaded_from_swap) =
        resolve_content_on_open(path, &work_dir, "").expect("resolve");

    assert_eq!(content, "");
    assert!(!loaded_from_swap);
}

#[test]
fn untitled_resolve_recovers_from_swap() {
    let work_dir = test_work_dir("untitled_with_swap");
    let path = "untitled/Untitled-2";
    let saved = "draft content\nline two";

    write_swap(path, &work_dir, saved).expect("write swap");

    let (content, loaded_from_swap) =
        resolve_content_on_open(path, &work_dir, "").expect("resolve");

    assert_eq!(content, saved);
    assert!(loaded_from_swap);
}

#[test]
fn untitled_delete_swap_clears_recovery() {
    let work_dir = test_work_dir("untitled_delete_swap");
    let path = "untitled/Untitled-3";

    write_swap(path, &work_dir, "temporary").expect("write swap");
    delete_swap(path, &work_dir).expect("delete swap");

    let (content, loaded_from_swap) =
        resolve_content_on_open(path, &work_dir, "").expect("resolve");

    assert_eq!(content, "");
    assert!(!loaded_from_swap);
}
