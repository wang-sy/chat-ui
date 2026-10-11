import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import {
  ArrowLeftIcon, ArrowUpIcon, CaretDownIcon, CaretRightIcon, ChatCircleIcon,
  CheckIcon, CircleNotchIcon, DesktopIcon, FolderIcon, FolderOpenIcon,
  GearSixIcon, HouseIcon, MagnifyingGlassIcon, PlusIcon, SidebarSimpleIcon,
  SparkleIcon, SquareIcon, TerminalIcon, WarningCircleIcon, XIcon,
} from "@phosphor-icons/react";
import { bridge, serviceUrl, type AgentRequest, type AppState, type Directory } from "./bridge";

const emptyState: AppState = { connected: false, available: false, issue: null, projects: [], chats: [], models: [], home: "" };
const pathName = (path: string) => path.split(/[\\/]/).filter(Boolean).slice(-1)[0] || path;
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);

function RequestCard({ request, onReply }: { request: AgentRequest; onReply: (body: Parameters<typeof bridge.reply>[0]) => Promise<void> }) {
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  async function reply(decision?: string) {
    setPending(true); setError("");
    try {
      await onReply({ id: request.id, decision, answers: decision === "decline" ? {} : Object.fromEntries(Object.entries(answers).map(([id, answer]) => [id, { answers: [answer] }])) });
    } catch (cause) { setError(errorText(cause)); } finally { setPending(false); }
  }
  const input = request.method === "item/tool/requestUserInput";
  const approval = ["item/commandExecution/requestApproval", "item/fileChange/requestApproval"].includes(request.method);
  const permissions = request.method === "item/permissions/requestApproval";
  return <section className="request-card">
    <strong>{input ? "需要你的选择" : "需要你的确认"}</strong>
    <p>{request.params.reason || (approval ? "Codex 请求执行以下操作。" : "当前请求需要在原有权限范围内继续。")}</p>
    {request.params.command && <pre>{request.params.command}</pre>}
    {request.params.grantRoot && <code>{request.params.grantRoot}</code>}
    {permissions && <pre>{JSON.stringify(request.params.permissions, null, 2)}</pre>}
    {input && request.params.questions?.map((question) => <label className="question" key={question.id}>
      <span>{question.question}</span>
      <input type={question.isSecret ? "password" : "text"} list={question.isSecret ? undefined : `options-${question.id}`} value={answers[question.id] || ""} onChange={(e) => setAnswers({ ...answers, [question.id]: e.target.value })} />
      <datalist id={`options-${question.id}`}>{question.options?.map((option) => <option key={option.label} value={option.label}>{option.description}</option>)}</datalist>
    </label>)}
    <div className="dialog-actions">
      <button type="button" disabled={pending} onClick={() => void reply("decline")}>{input ? "取消" : "拒绝"}</button>
      {(approval || permissions || input) && <button type="button" className="primary-button" disabled={pending || (input && request.params.questions?.some(q => !answers[q.id]?.trim()))} onClick={() => void reply("accept")}>{input ? "提交" : "允许本次"}</button>}
    </div>
    {error && <p className="inline-error" role="alert">{error}</p>}
  </section>;
}

export function App() {
  const [state, setState] = useState<AppState>(emptyState);
  const [serviceOnline, setServiceOnline] = useState(false);
  const [connecting, setConnecting] = useState(true);
  const [sidebarOpen, setSidebarOpen] = useState(() => window.innerWidth > 540);
  const [projectPath, setProjectPath] = useState(() => localStorage.getItem("chat-ui:project") || "");
  const [chatId, setChatId] = useState(() => localStorage.getItem("chat-ui:chat") || "");
  const [modelId, setModelId] = useState("");
  const [effort, setEffort] = useState("");
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [directory, setDirectory] = useState<Directory | null>(null);
  const [pathInput, setPathInput] = useState("");
  const [directoryLoading, setDirectoryLoading] = useState(false);
  const [directoryError, setDirectoryError] = useState("");
  const textarea = useRef<HTMLTextAreaElement>(null);
  const end = useRef<HTMLDivElement>(null);
  const generation = useRef(0);
  const pendingSubmission = useRef<{ key: string; id: string } | null>(null);

  const chat = state.chats.find(item => item.id === chatId);
  const cwd = chat?.cwd || projectPath;
  const project = state.projects.find(item => item.path === cwd);
  const model = state.models.find(item => item.model === modelId);

  const refresh = useCallback(async () => {
    const next = await bridge.state();
    setState(next); setServiceOnline(true);
    return next;
  }, []);

  const connect = useCallback(async () => {
    setConnecting(true); setError("");
    try { await refresh(); setState(await bridge.connect()); }
    catch (cause) { setError(errorText(cause)); }
    finally { setConnecting(false); }
  }, [refresh]);

  useEffect(() => {
    const current = ++generation.current;
    let timer: ReturnType<typeof setTimeout>;
    let seenOnline = false;
    async function poll() {
      try {
        const next = await bridge.state();
        if (generation.current !== current) return;
        setState(next); setServiceOnline(true);
        if (!seenOnline && next.available && !next.connected) {
          seenOnline = true;
          setConnecting(true);
          try {
            const connected = await bridge.connect();
            if (generation.current === current) { setState(connected); setError(""); }
          } catch (cause) { if (generation.current === current) setError(errorText(cause)); }
        }
        seenOnline = true;
      } catch {
        seenOnline = false;
        if (generation.current === current) { setServiceOnline(false); setState(previous => ({ ...previous, connected: false })); }
      }
      if (generation.current === current) setConnecting(false);
      if (generation.current === current) timer = setTimeout(() => void poll(), document.hidden ? 2500 : 650);
    }
    void poll();
    return () => { generation.current++; clearTimeout(timer); };
  }, []);

  useEffect(() => {
    if (model && !model.supportedReasoningEfforts?.some(option => option.reasoningEffort === effort)) {
      setEffort(model.defaultReasoningEffort || "");
    }
  }, [model, effort]);

  useEffect(() => {
    if (state.models.length && !state.models.some(item => item.model === modelId)) {
      const selected = state.models.find(item => item.isDefault) || state.models[0];
      setModelId(selected.model); setEffort(selected.defaultReasoningEffort || "");
    }
    if (!projectPath && state.projects.length) setProjectPath(state.projects[0].path);
  }, [state.models, state.projects, modelId, projectPath]);

  useEffect(() => { localStorage.setItem("chat-ui:project", projectPath); }, [projectPath]);
  useEffect(() => { localStorage.setItem("chat-ui:chat", chatId); }, [chatId]);
  useEffect(() => { end.current?.scrollIntoView({ block: "end", behavior: "smooth" }); }, [chat?.messages[chat.messages.length - 1]?.text, chatId]);

  function newChat() { setChatId(""); setDraft(""); setError(""); textarea.current?.focus(); }

  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") { event.preventDefault(); setChatId(""); setDraft(""); setError(""); textarea.current?.focus(); }
      if (event.key === "Escape") { setPickerOpen(false); setSettingsOpen(false); }
    }
    window.addEventListener("keydown", keydown); return () => window.removeEventListener("keydown", keydown);
  }, []);

  async function browse(path?: string) {
    setDirectoryLoading(true); setDirectoryError("");
    try { const next = await bridge.directories(path); setDirectory(next); setPathInput(next.path); }
    catch (cause) { setDirectoryError(errorText(cause)); }
    finally { setDirectoryLoading(false); }
  }

  function openPicker() { setPickerOpen(true); void browse(cwd || undefined); }

  async function chooseProject() {
    if (!directory) return;
    setDirectoryLoading(true);
    try { const selected = await bridge.addProject(directory.path); setProjectPath(selected.path); newChat(); await refresh(); setPickerOpen(false); }
    catch (cause) { setDirectoryError(errorText(cause)); }
    finally { setDirectoryLoading(false); }
  }

  async function openChat(id: string) {
    setChatId(id); setDraft(""); setError("");
    const selected = state.chats.find(item => item.id === id);
    if (selected) { setProjectPath(selected.cwd); setModelId(selected.model); }
    try { await bridge.openChat(id); await refresh(); } catch (cause) { setError(errorText(cause)); }
  }

  async function send(event?: FormEvent) {
    event?.preventDefault();
    if (!draft.trim() || sending || chat?.activeTurn) return;
    if (!cwd) { openPicker(); return; }
    setSending(true); setError("");
    const text = draft.trim();
    const key = JSON.stringify([chat?.id, cwd, modelId, effort, text]);
    if (pendingSubmission.current?.key !== key) pendingSubmission.current = { key, id: crypto.randomUUID() };
    try {
      const result = await bridge.send({ threadId: chat?.id, cwd, model: modelId, effort, text, requestId: pendingSubmission.current.id });
      pendingSubmission.current = null;
      setChatId(result.id); setDraft(previous => previous.trim() === text ? "" : previous); await refresh();
    } catch (cause) { setError(errorText(cause)); }
    finally { setSending(false); }
  }

  async function stop() {
    if (!chat) return;
    try { await bridge.interrupt(chat.id); await refresh(); } catch (cause) { setError(errorText(cause)); }
  }

  const connectionLabel = connecting ? "正在连接" : !serviceOnline ? "服务未连接" : state.connected ? "已连接 Codex" : state.available ? "连接失败" : "未找到 Codex";
  const filteredChats = state.chats.filter(item => (!cwd || item.cwd === cwd) && item.title.toLowerCase().includes(search.toLowerCase()));

  return <div className={`app-frame${sidebarOpen ? "" : " sidebar-collapsed"}`}>
    <aside className="workspace-sidebar" aria-label="侧边栏">
      <div className="sidebar-brand-row">
        <span className="product-name"><TerminalIcon weight="bold" size={21} />Chat UI</span>
        <button type="button" className="icon-button" title="收起侧边栏" aria-label="收起侧边栏" onClick={() => setSidebarOpen(false)}><SidebarSimpleIcon size={19}/></button>
      </div>
      <button type="button" className="nav-row new-chat" onClick={newChat}><PlusIcon size={18}/><span>新聊天</span><kbd>Ctrl N</kbd></button>
      <button type="button" className="nav-row" onClick={() => setSearchOpen(value => !value)}><MagnifyingGlassIcon size={17}/><span>搜索聊天</span></button>
      {searchOpen && <input autoFocus className="search-input" placeholder="搜索最近的聊天" aria-label="搜索最近的聊天" value={search} onChange={e => setSearch(e.target.value)} />}
      <div className="sidebar-section-heading"><span>项目</span><button className="icon-button" type="button" onClick={openPicker} title="添加项目" aria-label="添加项目"><PlusIcon size={15}/></button></div>
      <div className="project-list">{state.projects.map(item => <button type="button" className={`nav-row${item.path === cwd ? " is-selected" : ""}`} key={item.path} title={item.path} onClick={() => { setProjectPath(item.path); newChat(); }}><FolderIcon size={17}/><span>{item.name}</span>{item.path === cwd && <span className="selection-dot"/>}</button>)}</div>
      {!state.projects.length && <button className="empty-project" type="button" onClick={openPicker}>选择你的第一个项目<PlusIcon size={14}/></button>}
      <div className="sidebar-section-heading"><span>最近</span></div>
      <div className="recent-list">{filteredChats.map(item => <button className={`nav-row recent-row${item.id === chatId ? " is-selected" : ""}`} type="button" onClick={() => void openChat(item.id)} key={item.id} title={item.title}><ChatCircleIcon size={16}/><span>{item.title}</span>{item.activeTurn && <CircleNotchIcon className="spin" size={13}/>}</button>)}{!filteredChats.length && <p className="sidebar-empty">{search ? "没有匹配的聊天" : "开始聊天后会显示在这里"}</p>}</div>
      <div className="sidebar-footer"><button className="nav-row" type="button" onClick={() => setSettingsOpen(true)}><GearSixIcon size={18}/><span>设置</span><span className={`connection-dot${state.connected ? " is-live" : ""}`} /></button><small>在你的电脑上运行</small></div>
    </aside>

    <main className="conversation-shell">
      <header className="conversation-topbar">
        <div className="topbar-context">{!sidebarOpen && <button className="icon-button" title="展开侧边栏" aria-label="展开侧边栏" type="button" onClick={() => setSidebarOpen(true)}><SidebarSimpleIcon size={20}/></button>}<span>{chat ? chat.title : "新聊天"}</span></div>
        <button type="button" className="connection-button" onClick={() => setSettingsOpen(true)}><span className={`connection-dot${state.connected ? " is-live" : ""}`} />{connectionLabel}</button>
      </header>
      <section className={`landing-page${chat ? " has-messages" : ""}`}>
        {!chat ? <div className="welcome-block"><TerminalIcon className="welcome-icon" size={43} weight="light"/><h1>{project ? <>你想在 <button type="button" onClick={openPicker}>{project.name}</button> 中构建什么？</> : "从一个想法开始"}</h1></div> : <div className="conversation-list" aria-live="polite">
          {chat.messages.map(message => <article className={`message is-${message.role}`} key={message.id}>{message.role === "assistant" && <SparkleIcon size={20} className="assistant-icon"/>}<div className="message-content">{message.text}</div></article>)}
          {chat.activeTurn && <div className="working-status" role="status"><CircleNotchIcon size={15} className="spin"/>{chat.activity || "正在工作"}</div>}
          {!chat.activeTurn && chat.activity === "已停止" && <div className="working-status">已停止</div>}
          {chat.requests.map(request => <RequestCard key={request.id} request={request} onReply={async body => { await bridge.reply(body); await refresh(); }}/>) }
          {chat.error && <p className="inline-error" role="alert">{chat.error}</p>}
          <div ref={end}/>
        </div>}

        <div className="composer-area">
          {serviceOnline && !state.available && !connecting && <div className="status-notice" role="status"><WarningCircleIcon size={17}/><span>未在 PATH 中找到 Codex。安装或加入 PATH 后重启本地服务。</span><button type="button" onClick={() => void connect()}>重试</button></div>}
          {!serviceOnline && !connecting && <div className="status-notice" role="status"><WarningCircleIcon size={17}/><span>无法连接本地服务，请打开 Chat UI 桌面版或启动本地服务。</span><button type="button" onClick={() => void connect()}>重试</button></div>}
          {error && <p className="inline-error" role="alert">{error}</p>}
          <div className="composer-context"><button type="button" onClick={openPicker} disabled={Boolean(chat)} title={cwd || "选择项目目录"}><FolderIcon size={15}/><span>{project?.name || (cwd ? pathName(cwd) : "选择项目目录")}</span>{!chat && <CaretDownIcon size={11}/>}</button><span><DesktopIcon size={15}/>此计算机</span></div>
          <form className="composer" onSubmit={e => void send(e)}>
            <textarea ref={textarea} rows={3} aria-label="消息" placeholder={cwd ? "描述你想完成的事情" : "选择目录，开始一起工作"} value={draft} onChange={e => setDraft(e.target.value)} onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); void send(); } }} />
            <div className="composer-toolbar"><span className="permission-label"><CheckIcon size={13}/>项目内操作需遵守 Codex 权限</span><div className="toolbar-spacer"/>
              <label className="model-select"><SparkleIcon size={14}/><select aria-label="模型" disabled={!state.models.length || Boolean(chat?.activeTurn)} value={modelId} onChange={e => { setModelId(e.target.value); setEffort(state.models.find(m => m.model === e.target.value)?.defaultReasoningEffort || ""); }}>{!state.models.length && <option value="">{connecting ? "加载模型…" : "模型不可用"}</option>}{state.models.map(item => <option key={item.model} value={item.model}>{item.displayName || item.model}</option>)}</select><CaretDownIcon size={11}/></label>
              {model?.supportedReasoningEfforts?.length ? <label className="effort-select"><select aria-label="思考深度" value={effort} disabled={Boolean(chat?.activeTurn)} onChange={e => setEffort(e.target.value)}>{model.supportedReasoningEfforts.map(option => <option key={option.reasoningEffort} value={option.reasoningEffort}>{option.reasoningEffort}</option>)}</select></label> : null}
              {chat?.activeTurn ? <button className="send-button" type="button" aria-label="停止生成" onClick={() => void stop()}><SquareIcon size={13} weight="fill"/></button> : <button className="send-button" type="submit" aria-label="发送消息" disabled={!draft.trim() || !state.connected || !modelId || sending}>{sending ? <CircleNotchIcon className="spin" size={16}/> : <ArrowUpIcon size={18} weight="bold"/>}</button>}
            </div>
          </form>
        </div>
      </section>
    </main>

    {pickerOpen && <div className="modal-backdrop" onClick={() => setPickerOpen(false)}><section role="dialog" aria-modal="true" aria-labelledby="directory-title" className="directory-dialog" onClick={e => e.stopPropagation()}><header><h2 id="directory-title">选择项目目录</h2><button type="button" className="icon-button" aria-label="关闭目录选择" onClick={() => setPickerOpen(false)}><XIcon size={18}/></button></header>
      <form className="path-input" onSubmit={e => { e.preventDefault(); void browse(pathInput); }}><button type="button" className="icon-button" aria-label="上一级目录" disabled={!directory?.parent || directoryLoading} onClick={() => void browse(directory?.parent || undefined)}><ArrowLeftIcon size={17}/></button><input aria-label="目录路径" value={pathInput} onChange={e => setPathInput(e.target.value)} placeholder="输入目录路径"/><button type="submit" disabled={directoryLoading}>前往</button></form>
      <div className="folder-browser"><nav aria-label="目录快捷入口">{directory?.roots.map(root => <button type="button" key={root.path} onClick={() => void browse(root.path)}><HouseIcon size={16}/>{root.name}</button>)}</nav><div className="folder-list" aria-busy={directoryLoading}>{directoryLoading ? <p className="folder-placeholder">正在读取目录…</p> : directory?.children.length ? directory.children.map(item => <button key={item.path} type="button" onClick={() => void browse(item.path)}><FolderIcon size={18}/><span>{item.name}</span><CaretRightIcon size={13}/></button>) : <p className="folder-placeholder">此目录没有子文件夹</p>}</div></div>
      {directoryError && <p className="inline-error" role="alert">{directoryError}</p>}<footer><span title={directory?.path}><FolderOpenIcon size={16}/>{directory?.path || "请选择目录"}</span><button type="button" className="primary-button" disabled={!directory || directoryLoading} onClick={() => void chooseProject()}>选择此目录</button></footer>
    </section></div>}

    {settingsOpen && <div className="modal-backdrop" onClick={() => setSettingsOpen(false)}><section role="dialog" aria-modal="true" aria-labelledby="settings-title" className="settings-dialog" onClick={e => e.stopPropagation()}><header><h2 id="settings-title">设置</h2><button type="button" className="icon-button" aria-label="关闭设置" onClick={() => setSettingsOpen(false)}><XIcon size={18}/></button></header><div className="settings-body"><h3>本地 Codex</h3><p>浏览器和桌面版共享这台电脑上的服务与线程。</p><div className="setting-row"><span>连接状态</span><span>{connectionLabel}</span></div><div className="setting-row"><span>本地服务</span><a href={serviceUrl} target="_blank" rel="noreferrer">{serviceUrl}</a></div><div className="setting-row"><span>Codex</span><span>{state.available ? "已在 PATH 中找到" : "未在 PATH 中找到"}</span></div><p className="settings-note">使用本机 Codex 已有的登录和配置。Chat UI 不要求填写可执行文件路径。</p>{state.issue && <p className="inline-error">{state.issue}</p>}<button className="primary-button" type="button" disabled={connecting} onClick={() => void connect()}>{connecting ? "正在连接…" : "重新连接"}</button></div></section></div>}
  </div>;
}
