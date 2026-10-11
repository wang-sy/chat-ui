import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type BridgeStatus = {
  running: boolean;
  pid: number | null;
  executable: string | null;
};

const browserStatus: BridgeStatus = {
  running: false,
  pid: null,
  executable: null,
};

const hasTauriRuntime =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const bridge = {
  isNative: hasTauriRuntime,

  async start(executable?: string): Promise<BridgeStatus> {
    if (!hasTauriRuntime) return browserStatus;
    return invoke<BridgeStatus>("codex_start", { executable });
  },

  async stop(): Promise<BridgeStatus> {
    if (!hasTauriRuntime) return browserStatus;
    return invoke<BridgeStatus>("codex_stop");
  },

  async send(message: string): Promise<void> {
    if (!hasTauriRuntime) {
      throw new Error("The browser preview cannot send to a local Codex process.");
    }
    await invoke("codex_send", { message });
  },

  async subscribe(handler: (message: string) => void): Promise<() => void> {
    if (!hasTauriRuntime) return () => undefined;
    return listen<string>("codex://message", (event) => handler(event.payload));
  },
};
