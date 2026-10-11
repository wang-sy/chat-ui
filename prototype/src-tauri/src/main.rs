#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};

struct CodexProcess {
    child: Child,
    stdin: ChildStdin,
    executable: String,
}

#[derive(Default)]
struct BridgeState {
    process: Mutex<Option<CodexProcess>>,
}

#[derive(Debug, Serialize)]
struct BridgeStatus {
    running: bool,
    pid: Option<u32>,
    executable: Option<String>,
}

impl BridgeStatus {
    fn stopped() -> Self {
        Self {
            running: false,
            pid: None,
            executable: None,
        }
    }
}

#[tauri::command]
fn codex_start(
    app: AppHandle,
    state: State<'_, BridgeState>,
    executable: Option<String>,
) -> Result<BridgeStatus, String> {
    let executable = executable
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_codex_executable);

    let mut command = Command::new(&executable);
    command
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start Codex at {executable}: {error}"))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Codex stdin was not available".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Codex stdout was not available".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Codex stderr was not available".to_string())?;
    let pid = child.id();

    let mut process = state
        .process
        .lock()
        .map_err(|_| "Codex bridge state is unavailable".to_string())?;

    if process.is_some() {
        let _ = child.kill();
        return Err("A Codex app-server is already running".to_string());
    }

    *process = Some(CodexProcess {
        child,
        stdin,
        executable: executable.clone(),
    });
    drop(process);

    spawn_reader(app.clone(), stdout, false);
    spawn_reader(app, stderr, true);

    Ok(BridgeStatus {
        running: true,
        pid: Some(pid),
        executable: Some(executable),
    })
}

#[tauri::command]
fn codex_send(state: State<'_, BridgeState>, message: String) -> Result<(), String> {
    let mut process = state
        .process
        .lock()
        .map_err(|_| "Codex bridge state is unavailable".to_string())?;
    let process = process
        .as_mut()
        .ok_or_else(|| "Codex app-server is not running".to_string())?;

    process
        .stdin
        .write_all(message.trim_end().as_bytes())
        .and_then(|_| process.stdin.write_all(b"\n"))
        .and_then(|_| process.stdin.flush())
        .map_err(|error| format!("failed to write to Codex: {error}"))
}

#[tauri::command]
fn codex_stop(state: State<'_, BridgeState>) -> Result<BridgeStatus, String> {
    let mut process = state
        .process
        .lock()
        .map_err(|_| "Codex bridge state is unavailable".to_string())?;

    if let Some(mut process) = process.take() {
        let _ = process.child.kill();
        let _ = process.child.wait();
    }

    Ok(BridgeStatus::stopped())
}

#[tauri::command]
fn codex_status(state: State<'_, BridgeState>) -> Result<BridgeStatus, String> {
    let mut process = state
        .process
        .lock()
        .map_err(|_| "Codex bridge state is unavailable".to_string())?;

    let exited = match process.as_mut() {
        Some(process) => process
            .child
            .try_wait()
            .map_err(|error| format!("failed to read Codex status: {error}"))?
            .is_some(),
        None => false,
    };

    if exited {
        *process = None;
        return Ok(BridgeStatus::stopped());
    }

    Ok(match process.as_ref() {
        Some(process) => BridgeStatus {
            running: true,
            pid: Some(process.child.id()),
            executable: Some(process.executable.clone()),
        },
        None => BridgeStatus::stopped(),
    })
}

fn spawn_reader<R>(app: AppHandle, reader: R, stderr: bool)
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines().map_while(Result::ok) {
            let payload = if stderr {
                format!("[stderr] {line}")
            } else {
                line
            };
            let _ = app.emit("codex://message", payload);
        }
    });
}

fn default_codex_executable() -> String {
    if cfg!(windows) {
        "codex.exe".to_string()
    } else {
        "codex".to_string()
    }
}

fn main() {
    tauri::Builder::default()
        .manage(BridgeState::default())
        .invoke_handler(tauri::generate_handler![
            codex_start,
            codex_send,
            codex_stop,
            codex_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chat UI");
}
