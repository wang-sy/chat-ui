# Chat UI

Chat UI is a Codex-style local workspace. One Rust service owns one local Codex app-server session, while the browser and Tauri desktop window use the same projects, chats, thread IDs, approvals, and streaming state.

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
