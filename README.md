# Chat UI

Chat UI is a local visual workspace powered by Codex. One Rust service owns one local Codex app-server session, while the browser and Tauri desktop window use the same projects, Spaces, chats, thread IDs, approvals, and streaming state.

## Run locally

From `prototype/`:

```powershell
pnpm install
pnpm dev
```

Open `http://127.0.0.1:1420/` for the browser client. The command starts the Vite UI and the local Rust service on `127.0.0.1:4317` when the service is not already running.

For the desktop client:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
pnpm tauri dev
```

The desktop window starts the same local service. To use the packaged browser surface after a release build, open `http://127.0.0.1:4317/` while the desktop service is running.

Chat UI discovers `codex`, `codex.exe`, `codex.cmd`, or `codex.bat` from `PATH`. If it is missing, the UI shows a notice; there is no executable-path field to configure.

## Build and verify

```powershell
pnpm test
pnpm test:codex
pnpm build
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
pnpm tauri build
```

The Windows application is written to `prototype/src-tauri/target/release/chat-ui.exe`. Project registration is stored in the user `.chat-ui` directory; each project keeps its chat state in `.space/chat-ui.json`.

See [SPEC.md](SPEC.md) for the product model and local thread protocol.
See [CODEX_CAPABILITIES.md](CODEX_CAPABILITIES.md) for the app-server capability matrix and integration boundaries.

## Workspace model

The browser opens a project-level Space rather than a message-only screen. The Space contains one Main Thread and a registry of isolated View pages. Main Thread turns return a complete composition plan (`mainPage`, `views`, `connections`, and explicit `removeViews`); the Rust runtime turns that plan into child View Threads and persists the generated Main Page and each View below `.space/main/` and `.space/views/<view-id>/`. The UI presents one page surface at a time: the project opens Main Page, while selecting a View Thread swaps that surface to the View page. A compact bottom conversation dock keeps pending messages visible and expands for the active thread's approvals, streamed output and errors.

Space state is stored in the selected project's `.space/space.json`; thread messages remain in `.space/chat-ui.json`. The service is the only owner of these files, so browser and Tauri clients observe the same workspace.
