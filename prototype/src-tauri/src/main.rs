#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chat_ui::{runtime::hidden_command, server};
use std::process::Stdio;

fn main() {
    if std::env::args().any(|arg| arg == "--service") {
        let port = std::env::var("CHAT_UI_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(server::PORT);
        if let Err(error) = server::serve(port, server::state_directory()) {
            eprintln!("Chat UI service: {error}");
            std::process::exit(1);
        }
        return;
    }

    tauri::Builder::default()
        .setup(|_| {
            if !server::is_running(server::PORT) {
                let executable = std::env::current_exe()?;
                hidden_command(&executable)
                    .arg("--service")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Chat UI");
}
