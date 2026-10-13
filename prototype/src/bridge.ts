export type PermissionMode = "ask" | "auto" | "full";

export type Model = {
  model: string;
  displayName: string;
  description?: string;
  isDefault: boolean;
  defaultReasoningEffort: string;
  supportedReasoningEfforts: { reasoningEffort: string; description: string }[];
  serviceTiers?: { id: string; name: string; description: string }[];
  /** Additional response-speed tiers exposed by the Codex model catalog. */
  additionalSpeedTiers?: string[];
  defaultServiceTier?: string | null;
};

export type Message = { id: string; role: "user" | "assistant"; text: string };
export type TranscriptItem = {
  id: string;
  turnId: string;
  kind: "user" | "assistant" | "reasoning" | "command" | "file" | "tool" | "plan" | "notice";
  title: string;
  text: string;
  status: "running" | "completed" | "failed" | "interrupted" | string;
};
export type AgentRequest = {
  id: string | number;
  method: string;
  params: {
    reason?: string;
    command?: string | string[];
    grantRoot?: string;
    permissions?: Record<string, unknown>;
    message?: string;
    serverName?: string;
    mode?: "form" | "openai/form" | "openaiForm" | "url" | string;
    url?: string;
    elicitationId?: string;
    requestedSchema?: {
      properties?: Record<
        string,
        {
          title?: string;
          type?: string;
          format?: string;
          enum?: string[];
        }
      >;
      required?: string[];
      [key: string]: unknown;
    };
    _meta?: unknown;
    questions?: {
      id: string;
      question: string;
      isSecret?: boolean;
      options?: { label: string; description?: string }[];
    }[];
  };
};
export type Chat = {
  id: string;
  title: string;
  cwd: string;
  model: string;
  effort: string;
  speed?: string;
  permission: PermissionMode | string;
  role?: "main" | "view" | string;
  viewId?: string | null;
  parentThreadId?: string | null;
  messages: Message[];
  transcript?: TranscriptItem[];
  activeTurn: string | null;
  activity: string | null;
  activityDetail?: string | null;
  error: string | null;
  turnPlan?: TurnPlan | null;
  tokenUsage?: TokenUsage | null;
  requests: AgentRequest[];
};
export type TurnPlanStep = {
  step: string;
  status: "pending" | "inProgress" | "completed" | string;
};
export type TurnPlan = {
  explanation?: string | null;
  plan?: TurnPlanStep[];
};
export type TokenUsageBreakdown = {
  totalTokens?: number;
  inputTokens?: number;
  outputTokens?: number;
  reasoningOutputTokens?: number;
};
export type TokenUsage = {
  last?: TokenUsageBreakdown;
  total?: TokenUsageBreakdown;
  modelContextWindow?: number | null;
};
export type Project = { path: string; name: string };
export type ViewRecord = {
  id: string;
  key?: string;
  title: string;
  kind: string;
  threadId: string | null;
  status: string;
  brief?: string;
  summary?: string;
  x: number;
  y: number;
  width: number;
  height: number;
  collapsed: boolean;
  version: number;
  html: string;
  updatedAt: string;
};
export type MainPageRecord = {
  title: string;
  summary: string;
  html: string;
  version: number;
  updatedAt: string;
};
export type ViewConnection = {
  id: string;
  from: string;
  to: string;
  label: string | null;
};
export type Space = {
  id: string;
  title: string;
  cwd: string;
  revision: number;
  mainThreadId: string | null;
  mainPage: MainPageRecord;
  selectedView: string | null;
  views: ViewRecord[];
  connections: ViewConnection[];
};
export type AppState = {
  instanceId?: string;
  revision?: number;
  connected: boolean;
  available: boolean;
  issue: string | null;
  projects: Project[];
  chats: Chat[];
  models: Model[];
  home: string;
};
export type StreamEvent = {
  sequence: number;
  method: string;
  params: Record<string, unknown>;
  state: AppState;
};
export type Directory = {
  path: string;
  parent: string | null;
  children: { name: string; path: string }[];
  roots: { name: string; path: string }[];
};

export const serviceUrl = "http://127.0.0.1:4317";
let token: string | undefined;
let pendingToken: Promise<string> | undefined;

async function getToken(): Promise<string> {
  if (token) return token;
  if (!pendingToken) {
    pendingToken = (async () => {
      const response = await fetch(`${serviceUrl}/health`, {
        signal: AbortSignal.timeout(5000),
      });
      const body = await response.json();
      if (
        !response.ok ||
        body.service !== "chat-ui" ||
        body.protocol !== 1 ||
        typeof body.token !== "string"
      ) {
        throw new Error("本地端口没有运行兼容的 Chat UI 服务");
      }
      token = body.token as string;
      return token;
    })().finally(() => {
      pendingToken = undefined;
    });
  }
  return pendingToken;
}

async function request<T>(
  path: string,
  body?: unknown,
  retry = true,
): Promise<T> {
  const auth = await getToken();
  const response = await fetch(`${serviceUrl}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      "X-Chat-UI-Token": auth,
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(body === undefined ? 10000 : 120000),
  });
  if (response.status === 401 && retry) {
    token = undefined;
    return request(path, body, false);
  }
  const text = await response.text();
  let result: unknown;
  try {
    result = JSON.parse(text);
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    const message =
      result && typeof result === "object" && "error" in result
        ? result.error
        : undefined;
    throw new Error(
      typeof message === "string"
        ? message
        : `本地服务错误 ${response.status}`,
    );
  }
  if (result === undefined) throw new Error("本地服务返回了无效响应");
  return result as T;
}

export const bridge = {
  state: () => request<AppState>("/api/state"),
  connect: () => request<AppState>("/api/connect", {}),
  directories: (path?: string) =>
    request<Directory>(
      `/api/directories${path ? `?path=${encodeURIComponent(path)}` : ""}`,
    ),
  createDirectory: (parent: string, name: string) =>
    request<Directory>("/api/directories", { parent, name }),
  addProject: (path: string) =>
    request<{ path: string }>("/api/projects", { path }),
  space: (cwd: string) =>
    request<Space>(`/api/space?cwd=${encodeURIComponent(cwd)}`),
  mutateSpace: (body: {
    cwd: string;
    action?: string;
    [key: string]: unknown;
  }) => request<Space>("/api/space", body),
  openChat: (id: string) => request<{ id: string }>("/api/chats/open", { id }),
  send: (body: {
    threadId?: string;
    viewId?: string;
    cwd: string;
    model: string;
    effort: string;
    speed?: string;
    permission: PermissionMode;
    text: string;
    requestId: string;
  }) => request<{ id: string }>("/api/chats/send", body),
  steer: (body: {
    threadId: string;
    cwd: string;
    text: string;
    requestId: string;
  }) => request<{ id: string; steered: boolean }>("/api/chats/steer", body),
  interrupt: (id: string) => request("/api/chats/interrupt", { id }),
  reply: (body: {
    id: string | number;
    decision?: string;
    answers?: Record<string, { answers: string[] }>;
    content?: Record<string, unknown> | null;
  }) => request("/api/requests/reply", body),
  subscribeEvents: async (onEvent: (event: StreamEvent) => void) => {
    let source: EventSource | undefined;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    let closed = false;
    let lastSequence = 0;
    function scheduleReconnect() {
      if (closed) return;
      clearTimeout(reconnectTimer);
      reconnectTimer = setTimeout(() => {
        void open().catch(scheduleReconnect);
      }, 600);
    }
    async function open() {
      if (closed) return;
      const auth = await getToken();
      const since = lastSequence ? `&since=${lastSequence}` : "";
      const opened = new EventSource(
        `${serviceUrl}/api/events?token=${encodeURIComponent(auth)}${since}`,
      );
      source = opened;
      opened.onmessage = (event) => {
        if (closed || source !== opened) return;
        try {
          const payload = JSON.parse(event.data) as StreamEvent;
          const eventSequence = Number(event.lastEventId);
          const sequence =
            typeof payload?.sequence === "number"
              ? payload.sequence
              : Number.isFinite(eventSequence)
                ? eventSequence
                : 0;
          if (payload && sequence > 0 && payload.state) {
            lastSequence = sequence;
            onEvent(payload);
          }
        } catch {
          // A malformed event should not take down the fallback state poller.
        }
      };
      opened.onerror = () => {
        if (closed || source !== opened) return;
        opened.close();
        source = undefined;
        token = undefined;
        scheduleReconnect();
      };
    }
    await open();
    return () => {
      closed = true;
      clearTimeout(reconnectTimer);
      source?.close();
      source = undefined;
    };
  },
};
