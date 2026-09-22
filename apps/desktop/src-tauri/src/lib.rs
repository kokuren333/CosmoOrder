//! Osmium desktop shell.
//!
//! The shell owns no learning semantics. It opens one application session over
//! the user's data directory and exposes that session to the React renderer
//! through typed commands. The renderer cannot read a filesystem path, run
//! package content, or reach the network: the capability set grants only this
//! application's own commands.

// The Windows linker reports creating the cdylib import library on every build.
// It is expected output for a `cdylib` crate, not a defect in this code.
#![allow(linker_messages)]

pub mod commands;
pub mod error;

use commands::Desktop;
use osmium_package::library::default_home;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Data directory used by this process. `OSMIUM_HOME` wins, then the platform
/// default, matching the CLI exactly.
fn data_home() -> Result<std::path::PathBuf, error::CommandError> {
    default_home().map_err(error::CommandError::new)
}

/// Remote debugging is opt-in and never enabled implicitly, so a normal run
/// exposes no developer endpoint. The end-to-end test sets it to inspect the
/// real DOM of the packaged application.
fn remote_debugging_args() -> Option<String> {
    let port = std::env::var("OSMIUM_DEBUG_PORT").ok()?;
    let port: u16 = port.parse().ok()?;
    Some(format!("--remote-debugging-port={port}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let home = match data_home() {
        Ok(home) => home,
        Err(error) => {
            // Failing before the window exists is the honest outcome: there is
            // no data root to read or write.
            eprintln!("osmium: {error}");
            std::process::exit(2);
        }
    };
    let desktop = match Desktop::open(home) {
        Ok(desktop) => desktop,
        Err(error) => {
            eprintln!("osmium: {error}");
            std::process::exit(3);
        }
    };
    tauri::Builder::default()
        .manage(desktop)
        .setup(|app| {
            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Osmium")
                    .inner_size(1180.0, 820.0)
                    .min_inner_size(720.0, 520.0)
                    .resizable(true)
                    .center();
            if let Some(arguments) = remote_debugging_args() {
                builder = builder.additional_browser_args(&arguments);
            }
            builder.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::list_packages,
            commands::open_lesson,
            commands::read_resource,
            commands::submit_attempt,
            commands::progress,
            commands::history,
            commands::rebuild_progress,
            commands::export_state,
            commands::backup_state,
        ])
        .run(tauri::generate_context!())
        .expect("the Osmium desktop runtime starts");
}
