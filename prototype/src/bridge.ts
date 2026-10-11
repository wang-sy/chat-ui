export type Model = {
  model: string;
  displayName: string;
  isDefault: boolean;
  defaultReasoningEffort: string;
  supportedReasoningEfforts: { reasoningEffort: string; description: string }[];
};

export type Message = { id: string; role: "user" | "assistant"; text: string };
export type AgentRequest = {
  id: string | number;
  method: string;
  params: {
    reason?: string;
    command?: string;
    grantRoot?: string;
    permissions?: Record<string, unknown>;
    questions?: { id: string; question: string; isSecret?: boolean; options?: { label: string; description?: string }[] }[];
  };
};
export type Chat = {
  id: string; title: string; cwd: string; model: string;
  messages: Message[]; activeTurn: string | null; activity: string | null;
  error: string | null; requests: AgentRequest[];
};
export type Project = { path: string; name: string };
export type AppState = {
  connected: boolean; available: boolean; issue: string | null;
  projects: Project[]; chats: Chat[]; models: Model[]; home: string;
};
export type Directory = {
  path: string; parent: string | null;
  children: { name: string; path: string }[];
  roots: { name: string; path: string }[];
};

// The same HTTP client is used by the browser and the Tauri window.
export const serviceUrl = "http://127.0.0.1:4317";
let token: string | undefined;
let pendingToken: Promise<string> | undefined;

async function getToken(): Promise<string> {
  if (token) return token;
  if (!pendingToken) {
    pendingToken = (async () => {
      const response = await fetch(`${serviceUrl}/health`, { signal: AbortSignal.timeout(5000) });
      const body = await response.json();
      if (!response.ok || body.service !== "chat-ui" || body.protocol !== 1 || typeof body.token !== "string") {
        throw new Error("本地端口没有运行兼容的 Chat UI 服务");
      }
      token = body.token;
      return body.token as string;
    })().finally(() => { pendingToken = undefined; });
  }
  return pendingToken;
}

async function request<T>(path: string, body?: unknown, retry = true): Promise<T> {
  const auth = await getToken();
  const response = await fetch(`${serviceUrl}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "X-Chat-UI-Token": auth, ...(body === undefined ? {} : { "Content-Type": "application/json" }) },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(body === undefined ? 10000 : 120000),
  });
  if (response.status === 401 && retry) {
    token = undefined;
    return request(path, body, false);
  }
  const result = await response.json();
  if (!response.ok) throw new Error(typeof result.error === "string" ? result.error : `本地服务错误 ${response.status}`);
  return result as T;
}

export const bridge = {
  state: () => request<AppState>("/api/state"),
  connect: () => request<AppState>("/api/connect", {}),
  directories: (path?: string) => request<Directory>(`/api/directories${path ? `?path=${encodeURIComponent(path)}` : ""}`),
  addProject: (path: string) => request<{ path: string }>("/api/projects", { path }),
  openChat: (id: string) => request<{ id: string }>("/api/chats/open", { id }),
  send: (body: { threadId?: string; cwd: string; model: string; effort: string; text: string; requestId: string }) => request<{ id: string }>("/api/chats/send", body),
  interrupt: (id: string) => request("/api/chats/interrupt", { id }),
  reply: (body: { id: string | number; decision?: string; answers?: Record<string, { answers: string[] }> }) => request("/api/requests/reply", body),
};
