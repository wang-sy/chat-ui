import { FormEvent, useEffect, useMemo, useState } from "react";
import { bridge, type BridgeStatus } from "./bridge";

type LogEntry = {
  direction: "in" | "out" | "system";
  text: string;
};

const emptyStatus: BridgeStatus = {
  running: false,
  pid: null,
  executable: null,
};

export function App() {
  const [status, setStatus] = useState<BridgeStatus>(emptyStatus);
  const [executable, setExecutable] = useState("");
  const [message, setMessage] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [error, setError] = useState("");

  const modeLabel = useMemo(
    () => (bridge.isNative ? "Tauri runtime" : "Browser preview"),
    [],
  );

  useEffect(() => {
    let dispose: (() => void) | undefined;

    void bridge.subscribe((text) => {
      setLogs((current) => [...current.slice(-39), { direction: "in", text }]);
    }).then((cleanup) => {
      dispose = cleanup;
    });

    return () => dispose?.();
  }, []);

  async function handleStart() {
    setError("");
    try {
      const nextStatus = await bridge.start(executable.trim() || undefined);
      setStatus(nextStatus);
      setLogs((current) => [
        ...current.slice(-39),
        {
          direction: "system",
          text: nextStatus.running
            ? `Codex app-server started${nextStatus.pid ? ` (pid ${nextStatus.pid})` : ""}.`
            : "Browser preview does not start a local process.",
        },
      ]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleStop() {
    setError("");
    try {
      const nextStatus = await bridge.stop();
      setStatus(nextStatus);
      setLogs((current) => [
        ...current.slice(-39),
        { direction: "system", text: "Codex app-server stopped." },
      ]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleSend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextMessage = message.trim();
    if (!nextMessage) return;

    setError("");
    try {
      await bridge.send(nextMessage);
      setLogs((current) => [
        ...current.slice(-39),
        { direction: "out", text: nextMessage },
      ]);
      setMessage("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  return (
    <main className="chat-ui-shell">
      <header className="chat-ui-header">
        <div>
          <p className="eyebrow">CHAT UI RUNTIME</p>
          <h1>One workspace, many views.</h1>
        </div>
        <div className="runtime-badge">
          <span className={status.running ? "status-dot is-live" : "status-dot"} />
          {status.running ? "Codex connected" : modeLabel}
        </div>
      </header>

      <section className="surface" aria-label="Main Page surface">
        <div className="surface-toolbar">
          <div>
            <span className="surface-kicker">MAIN PAGE</span>
            <strong>Main Thread surface</strong>
          </div>
          <span className="surface-note">Generated HTML lives here.</span>
        </div>
        <div className="view-grid">
          <article className="view-card view-card-main">
            <span className="view-label">MAIN THREAD</span>
            <h2>The host stays thin.</h2>
            <p>
              Main Thread will eventually compose this page and mount child Views
              without being constrained to a fixed dashboard.
            </p>
          </article>
          <article className="view-card">
            <span className="view-label">VIEW 01</span>
            <h2>Waiting for a thread</h2>
            <p>A child thread can deliver an independent HTML page into this frame.</p>
          </article>
        </div>
      </section>

      <section className="bridge-panel" aria-label="Codex bridge">
        <div className="bridge-heading">
          <div>
            <span className="surface-kicker">BRIDGE</span>
            <h2>Local Codex connection</h2>
          </div>
          <span className="bridge-state">{status.running ? "running" : "stopped"}</span>
        </div>
        <div className="bridge-controls">
          <input
            value={executable}
            onChange={(event) => setExecutable(event.target.value)}
            placeholder="Codex executable path (optional)"
            aria-label="Codex executable path"
          />
          <button type="button" onClick={handleStart} disabled={status.running}>
            Start
          </button>
          <button type="button" className="button-quiet" onClick={handleStop} disabled={!status.running}>
            Stop
          </button>
        </div>
        <form className="message-row" onSubmit={handleSend}>
          <input
            value={message}
            onChange={(event) => setMessage(event.target.value)}
            placeholder="Send a JSON-RPC line to the bridge"
            aria-label="JSON-RPC message"
          />
          <button type="submit">Send</button>
        </form>
        {error ? <p className="error-message">{error}</p> : null}
        <div className="log-list" aria-live="polite">
          {logs.length === 0 ? (
            <p className="log-empty">Bridge events will appear here.</p>
          ) : (
            logs.map((entry, index) => (
              <div className={`log-entry log-${entry.direction}`} key={`${entry.text}-${index}`}>
                <span>{entry.direction}</span>
                <code>{entry.text}</code>
              </div>
            ))
          )}
        </div>
      </section>
    </main>
  );
}
