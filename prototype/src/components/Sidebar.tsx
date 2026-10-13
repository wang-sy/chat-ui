import { PlusIcon, StackIcon } from "@phosphor-icons/react";
import type { AppState, Space, ViewRecord } from "../bridge";
import { ChatUiLogo } from "./ChatUiLogo";

export function Sidebar({
  state,
  cwd,
  space,
  activeSurface,
  activeViewId,
  onProject,
  onOpenChat,
  onOpenView,
}: {
  state: AppState;
  cwd: string;
  space: Space | null;
  activeSurface: "page" | "thread";
  activeViewId: string | null;
  onProject: (path: string) => void;
  onOpenChat: () => void;
  onOpenView: (view: ViewRecord) => void;
}) {
  return (
    <aside className="workspace-sidebar">
      <header className="sidebar-header">
        <div className="brand-button" aria-label="Chat UI">
          <span className="brand-mark">
            <ChatUiLogo size={27} />
          </span>
          <strong>Chat UI</strong>
        </div>
      </header>
      <div className="sidebar-scroll">
        <div className="sidebar-section-header">
          <span className="sidebar-section-label">Space</span>
          <button
            type="button"
            className="space-add-button"
            aria-label="添加 Space"
            title="添加 Space"
            onClick={() => onProject("")}
          >
            <PlusIcon size={16} />
          </button>
        </div>
        {state.projects.map((project) => (
          <div key={project.path} className="sidebar-project-group">
            <button
              type="button"
              className={`sidebar-project${project.path === cwd ? " is-selected" : ""}`}
              onClick={() => onProject(project.path)}
            >
              <StackIcon size={17} />
              <span>{project.name}</span>
            </button>
            {project.path === cwd && (
              <div className="sidebar-project-views">
                <button
                  type="button"
                  className={`sidebar-view-link is-main${activeSurface === "thread" && !activeViewId ? " is-active" : ""}`}
                  onClick={onOpenChat}
                >
                  <span className="view-link-dot" />
                  Main Thread
                </button>
                {space?.views.map((view) => (
                  <button
                    type="button"
                    className={`sidebar-view-link${activeSurface === "thread" && activeViewId === view.id ? " is-active" : ""}`}
                    key={view.id}
                    onClick={() => onOpenView(view)}
                  >
                    <span className={`view-link-dot is-${view.status}`} />
                    {view.title}
                  </button>
                ))}
              </div>
            )}
          </div>
        ))}
        {!state.projects.length && (
          <button
            type="button"
            className="sidebar-empty-cta"
            onClick={() => onProject("")}
          >
            添加第一个 Space
            <PlusIcon size={15} />
          </button>
        )}
      </div>
    </aside>
  );
}
