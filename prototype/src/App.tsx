import { useState } from "react";
import { PlusIcon, StackIcon, XIcon } from "@phosphor-icons/react";
import { ChatComposer } from "./components/ChatComposer";
import { ConversationPanel } from "./components/ConversationPanel";
import { RunningStatus } from "./components/ExecutionStatus";
import { DirectoryDialog } from "./components/DirectoryDialog";
import { Sidebar } from "./components/Sidebar";
import { MainPageSurface } from "./components/MainPageSurface";
import type { ViewRecord } from "./bridge";
import { useWorkspace } from "./hooks/useWorkspace";
import { useDirectoryBrowser } from "./hooks/useDirectoryBrowser";
import { useConversation } from "./hooks/useConversation";
import { useModelSettings } from "./hooks/useModelSettings";
import "./styles.css";

export function App() {
  const workspace = useWorkspace();
  const {
    state,
    cwd,
    space,
    setCwd,
    refresh,
    loadSpace,
    mutateSpace,
    setError,
  } = workspace;
  const directory = useDirectoryBrowser(async (path) => {
    setCwd(path);
    await refresh();
  });
  const conversation = useConversation({
    cwd,
    space,
    chats: state.chats,
    refresh,
    loadSpace,
    onError: setError,
    onChooseProject: () => directory.show(cwd || undefined),
  });
  const settings = useModelSettings(state.models, conversation.chat);
  const [permissionOpen, setPermissionOpen] = useState(false);
  const [surface, setSurface] = useState<"page" | "thread">("page");
  function selectProject(path: string) {
    if (!path) {
      directory.show(cwd || undefined);
      return;
    }
    setCwd(path);
    setSurface("page");
    conversation.openMain();
    conversation.setOpen(false);
  }

  function openMainThread() {
    setSurface("thread");
    conversation.openMain();
    if (space?.selectedView) void mutateSpace({ action: "select", viewId: null });
  }

  function openViewThread(view: ViewRecord) {
    setSurface("thread");
    conversation.openView(view);
    if (space?.selectedView !== view.id)
      void mutateSpace({ action: "select", viewId: view.id });
  }

  return (
    <div className="app-frame">
      <div className="app-body">
        <Sidebar
          state={state}
          cwd={cwd}
          space={space}
          activeSurface={surface}
          activeViewId={conversation.targetView?.id || null}
          onProject={selectProject}
          onOpenChat={openMainThread}
          onOpenView={openViewThread}
        />
        <main className="workspace-main">
          {workspace.error && (
            <div className="service-banner" role="alert">
              <span>{workspace.error}</span>
              {!state.connected && (
                <button
                  type="button"
                  onClick={() => void workspace.reconnect()}
                >
                  重新连接
                </button>
              )}
              <button
                type="button"
                aria-label="关闭提示"
                onClick={() => setError("")}
              >
                <XIcon size={15} />
              </button>
            </div>
          )}
          <div className="workspace-content">
            {space?.cwd === cwd ? (
              <MainPageSurface
                key={cwd}
                space={space}
                selectedViewId={
                  surface === "thread" ? conversation.targetView?.id || null : null
                }
                onViewEvent={(view, event) =>
                  void conversation.routeEvent(view, event, settings)
                }
                onOpenView={(view) => { openViewThread(view); conversation.setOpen(false); }}
              />
            ) : (
              <div className="space-empty">
                <div className="space-empty-icon">
                  <StackIcon size={31} />
                </div>
                <h1>{cwd ? "正在打开 Space…" : "你的工作空间从这里开始"}</h1>
                <p>
                  选择一个项目，Main Thread 会在 Space 中组织可运行的 View。
                </p>
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => directory.show(cwd || undefined)}
                >
                  <PlusIcon size={17} />
                  选择项目目录
                </button>
              </div>
            )}
          </div>
          <div
            className={`page-running-layer${
              conversation.open
                ? " is-dock-open"
                : ""
            }`}
          >
            <RunningStatus
              chat={conversation.chat}
              onOpen={() => conversation.setOpen(true)}
            />
          </div>
          <ConversationPanel
            chat={conversation.chat}
            target={conversation.target}
            targetTitle={conversation.targetView?.title}
            open={conversation.open}
            queue={conversation.queue}
            onToggle={() => conversation.setOpen((value) => !value)}
            onGuideQueue={conversation.guideQueue}
            onEditQueue={conversation.editQueue}
            onDeleteQueue={conversation.deleteQueue}
            onReorderQueue={conversation.reorderQueue}
            onReply={conversation.reply}
            composer={
              <ChatComposer
                models={state.models}
                model={settings.model}
                effort={settings.effort}
                speed={settings.speed}
                permission={settings.permission}
                draft={conversation.draft}
                busy={conversation.busy}
                activeTurn={Boolean(conversation.chat?.activeTurn)}
                running={Boolean(
                  conversation.chat?.activeTurn ||
                    (conversation.chat?.activity &&
                      conversation.chat.activity !== "已停止"),
                )}
                selectedView={conversation.targetView}
                permissionOpen={permissionOpen}
                onDraft={conversation.setDraft}
                onModel={settings.setModelId}
                onEffort={settings.setEffort}
                onSpeed={settings.setSpeed}
                onPermissionOpen={setPermissionOpen}
                onPermission={settings.setPermission}
                onSend={() => void conversation.send(settings)}
                onStop={() => void conversation.stop()}
              />
            }
          />
        </main>
      </div>
      <DirectoryDialog
        open={directory.open}
        directory={directory.directory}
        loading={directory.loading}
        mutating={directory.mutating}
        error={directory.error}
        canBack={directory.canBack}
        canForward={directory.canForward}
        onBack={directory.back}
        onForward={directory.forward}
        onBrowse={(path) => void directory.browse(path)}
        onCreate={directory.create}
        onChoose={(path) => void directory.choose(path)}
        onClose={directory.close}
      />
    </div>
  );
}
