use std::path::Path;

fn main() {
    tauri_build::build();

    // Windows运行时需要WebView2Loader.dll在EXE同目录
    // 此build.rs将其从bin/复制到target/release/确保Tauri打包时包含
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();

        // OUT_DIR路径: target/{profile}/build/{pkg}-{hash}/out
        // 往上3级回到 target/{profile}/
        let target_dir = Path::new(&out_dir)
            .parent().unwrap()
            .parent().unwrap()
            .parent().unwrap();

        let src = Path::new(&manifest_dir).join("bin").join("WebView2Loader.dll");
        let dest = target_dir.join("WebView2Loader.dll");

        if src.exists() {
            std::fs::copy(&src, &dest).ok();
        }
    }
}
