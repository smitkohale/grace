//! The web UI (`ui/dist`), embedded into the binary for `grace serve`.

use grace_core::api::{UiAssets, UiFile};
use std::sync::Arc;

#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/dist"]
#[allow_missing = true]
struct Ui;

pub fn ui() -> UiAssets {
    Arc::new(|path: &str| {
        let file = Ui::get(path)?;
        Some(UiFile {
            bytes: file.data,
            mime: mime_for(path),
        })
    })
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "glb" => "model/gltf-binary",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
