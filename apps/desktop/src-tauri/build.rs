//! Tauri build hook.
//!
//! The renderer bundle under `apps/desktop/dist` is embedded into the binary,
//! and Cargo does not look at it by itself. Declaring the renderer inputs here
//! keeps the embedded UI and its sources in step: editing the React sources
//! invalidates the Rust build, so a stale bundle cannot silently ship.
//!
//! `npm run build` must run before `cargo build`; the desktop test suite checks
//! that the embedded bundle is present and current.

use std::path::Path;

fn track(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            track(&path);
        }
    }
}

fn main() {
    for relative in [
        "../src",
        "../index.html",
        "../package.json",
        "../vite.config.ts",
    ] {
        let path = Path::new(relative);
        println!("cargo:rerun-if-changed={relative}");
        if path.is_dir() {
            track(path);
        }
    }
    tauri_build::build()
}
