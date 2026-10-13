import { useRef } from "react";
import {
  ArrowCounterClockwiseIcon,
  CaretLeftIcon,
  CaretRightIcon,
  CheckIcon,
  LightningIcon,
} from "@phosphor-icons/react";
import type { Model } from "../bridge";
import {
  effortDescriptions,
  effortNames,
  modelName,
  speedServiceLabel,
  type ModelSpeed,
  type SpeedOption,
} from "../modelSettings";

export type ModelControlPanel = "main" | "models" | "speed";

type Props = {
  panel: ModelControlPanel;
  models: Model[];
  model: Model | undefined;
  effort: string;
  speed: ModelSpeed;
  selectedEffort: string;
  selectedModel: string;
  supported: string[];
  currentIndex: number;
  effortPercent: number;
  speedOptions: readonly SpeedOption[];
  dragging: boolean;
  setDragging: (value: boolean) => void;
  setPanel: (panel: ModelControlPanel | null) => void;
  onEffort: (value: string) => void;
  onSpeed: (value: ModelSpeed) => void;
  onReset: () => void;
  onSelectModel: (id: string) => void;
};

export function ModelControlPopover({
  panel,
  models,
  model,
  effort,
  speed,
  selectedEffort,
  selectedModel,
  supported,
  currentIndex,
  effortPercent,
  speedOptions,
  dragging,
  setDragging,
  setPanel,
  onEffort,
  onSpeed,
  onReset,
  onSelectModel,
}: Props) {
  const track = useRef<HTMLDivElement>(null);
  function setEffortFromPointer(clientX: number) {
    const bounds = track.current?.getBoundingClientRect();
    if (!bounds || supported.length < 2) return;
    const ratio = Math.min(
      1,
      Math.max(0, (clientX - bounds.left) / bounds.width),
    );
    onEffort(supported[Math.round(ratio * (supported.length - 1))]);
  }

  return (
    <div
      className={`model-control-popover is-${panel}${dragging ? " is-dragging" : ""}`}
      role="dialog"
      aria-label="模型、速度和推理强度"
    >
      {panel === "main" && (
        <div className="model-main-panel">
          <div className="model-control-header">
            <button
              type="button"
              className="control-speed-button"
              aria-label={`选择速度，当前为${speedServiceLabel(speed)}`}
              title="选择速度"
              onClick={() => setPanel("speed")}
            >
              <LightningIcon size={24} weight="fill" />
            </button>
            <div className="control-summary">
              <strong>{selectedEffort}</strong>
              <button
                type="button"
                className="control-model-link"
                onClick={() => setPanel("models")}
              >
                {selectedModel}
                <CaretRightIcon size={16} />
              </button>
            </div>
            <button
              type="button"
              className="control-reset-button"
              aria-label="重置速度和强度"
              title="重置"
              onClick={onReset}
            >
              <ArrowCounterClockwiseIcon size={22} />
            </button>
          </div>
          <div className="effort-control">
            <div
              className="effort-track"
              ref={track}
              role="slider"
              aria-label="推理强度"
              aria-valuemin={0}
              aria-valuemax={supported.length - 1}
              aria-valuenow={currentIndex}
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
                  event.preventDefault();
                  onEffort(supported[Math.max(0, currentIndex - 1)]);
                }
                if (event.key === "ArrowRight" || event.key === "ArrowUp") {
                  event.preventDefault();
                  onEffort(
                    supported[Math.min(supported.length - 1, currentIndex + 1)],
                  );
                }
              }}
              onPointerDown={(event) => {
                event.currentTarget.setPointerCapture(event.pointerId);
                setDragging(true);
                setEffortFromPointer(event.clientX);
              }}
              onPointerMove={(event) => {
                if (dragging) setEffortFromPointer(event.clientX);
              }}
              onPointerUp={(event) => {
                if (event.currentTarget.hasPointerCapture(event.pointerId))
                  event.currentTarget.releasePointerCapture(event.pointerId);
                setDragging(false);
              }}
              onPointerCancel={() => setDragging(false)}
            >
              <div
                className="effort-track-fill"
                style={{ width: `${Math.max(7, effortPercent)}%` }}
              />
              {Array.from({ length: 18 }, (_, index) => (
                <i
                  key={index}
                  style={{
                    left: `${4 + ((index * 37) % 91)}%`,
                    top: `${4 + ((index * 17) % 48)}%`,
                  }}
                />
              ))}
              <span
                className="effort-thumb"
                style={{
                  left: `${Math.min(96, Math.max(4, effortPercent))}%`,
                }}
              />
            </div>
            <div className="effort-values">
              {supported.map((item) => (
                <button
                  type="button"
                  key={item}
                  className={item === effort ? "is-current" : ""}
                  onClick={() => onEffort(item)}
                >
                  {effortNames[item] || item}
                </button>
              ))}
            </div>
          </div>
          <p className="effort-description">
            {effortDescriptions[effort] || "选择模型的推理强度"}
          </p>
        </div>
      )}
      {panel === "models" && (
        <div className="model-list-panel">
          <div className="model-panel-heading">
            <button
              type="button"
              aria-label="返回模型设置"
              onClick={() => setPanel("main")}
            >
              <CaretLeftIcon size={18} />
            </button>
            <h3>选择模型</h3>
            <button
              type="button"
              aria-label="关闭"
              onClick={() => setPanel(null)}
            >
              ×
            </button>
          </div>
          <div className="default-model">
            <strong>默认</strong>
            <span>推荐模型集</span>
          </div>
          <div
            className="model-option-list"
            role="listbox"
            aria-label="可用模型"
          >
            {models.map((item) => (
              <button
                type="button"
                className={`model-option${item.model === model?.model ? " is-current" : ""}`}
                role="option"
                aria-selected={item.model === model?.model}
                key={item.model}
                onClick={() => onSelectModel(item.model)}
              >
                <span>{modelName(item.displayName || item.model)}</span>
                {item.model === model?.model && (
                  <CheckIcon size={19} weight="bold" />
                )}
              </button>
            ))}
          </div>
        </div>
      )}
      {panel === "speed" && (
        <div className="speed-list-panel">
          <div className="model-panel-heading">
            <button
              type="button"
              aria-label="返回模型设置"
              onClick={() => setPanel("main")}
            >
              <CaretLeftIcon size={18} />
            </button>
            <h3>选择速度</h3>
            <button
              type="button"
              aria-label="关闭"
              onClick={() => setPanel(null)}
            >
              ×
            </button>
          </div>
          <div
            className="speed-option-list"
            role="listbox"
            aria-label="模型速度"
          >
            {speedOptions.map((item) => (
              <button
                type="button"
                className={`speed-option${item.id === speed ? " is-current" : ""}`}
                role="option"
                aria-selected={item.id === speed}
                key={item.id}
                onClick={() => {
                  onSpeed(item.id);
                  setPanel("main");
                }}
              >
                <span className="speed-option-copy">
                  <strong>{item.title}</strong>
                  <small>{item.detail}</small>
                </span>
                {item.id === speed && <CheckIcon size={19} weight="bold" />}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
