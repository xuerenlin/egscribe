fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=desktop/egscribe.png");

    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=EGSCRIBE_BUILD_TIME={build_time}");

    #[cfg(windows)]
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let png_path = manifest_dir.join("desktop/egscribe.png");
    let icon_path = out_dir.join("egscribe_app.ico");

    build_bmp_icon(&png_path, &icon_path);

    let rc_path = out_dir.join("app.rc");
    let icon_for_rc = icon_path.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(&rc_path, format!("1 ICON \"{icon_for_rc}\"\n"))
        .expect("write app.rc");

    embed_resource::compile(rc_path.to_str().expect("rc path utf-8"), embed_resource::NONE);
}

/// rc.exe does not reliably embed PNG-compressed ICO entries; generate BMP-based ICO.
#[cfg(windows)]
fn build_bmp_icon(png_path: &std::path::Path, out_path: &std::path::Path) {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};

    let img = image::open(png_path)
        .unwrap_or_else(|e| panic!("failed to open {}: {e}", png_path.display()))
        .into_rgba8();

    let mut icon_dir = IconDir::new(ResourceType::Icon);
    for size in [16_u32, 32, 48, 256] {
        let resized = image::imageops::resize(
            &img,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let icon_image = IconImage::from_rgba_data(size, size, resized.into_raw());
        let entry = IconDirEntry::encode(&icon_image)
            .unwrap_or_else(|e| panic!("failed to encode {size}x{size} icon: {e}"));
        icon_dir.add_entry(entry);
    }

    let mut file = std::fs::File::create(out_path)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", out_path.display()));
    icon_dir
        .write(&mut file)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", out_path.display()));
}
