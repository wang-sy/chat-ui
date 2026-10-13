import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { CaretDownIcon, LightningIcon } from "@phosphor-icons/react";
import type { Model } from "../bridge";
import {
  efforts,
  effortNames,
  modelName,
  normalizeModelSpeed,
  speedServiceLabel,
  speedsForModel,
  type ModelSpeed,
} from "../modelSettings";
import {
  ModelControlPopover,
  type ModelControlPanel,
} from "./ModelControlPopover";

type Props = {
  models: Model[];
  model: Model | undefined;
  effort: string;
  speed: ModelSpeed;
  onModel: (id: string) => void;
  onEffort: (value: string) => void;
  onSpeed: (value: ModelSpeed) => void;
};

export function ModelPicker({
  models,
  model,
  effort,
  speed,
  onModel,
  onEffort,
  onSpeed,
}: Props) {
  const [panel, setPanel] = useState<ModelControlPanel | null>(null);
  const [dragging, setDragging] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const supported = useMemo(() => {
    const values = model?.supportedReasoningEfforts
      ?.map((item) => item.reasoningEffort)
      .filter(Boolean);
    return values?.length ? values : efforts;
  }, [model]);
  const speedOptions = useMemo(() => speedsForModel(model), [model]);
  const currentIndex = Math.max(0, supported.indexOf(effort));
  const effortPercent =
    supported.length > 1 ? (currentIndex / (supported.length - 1)) * 100 : 0;
  const selectedEffort = effortNames[effort] || effort || effortNames.medium;
  const selectedModel = modelName(
    model?.displayName || model?.model || "选择模型",
  );

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (root.current && !root.current.contains(event.target as Node)) {
        setPanel(null);
        setDragging(false);
      }
    };
    const escape = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        setPanel(null);
        setDragging(false);
      }
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", escape);
    };
  }, []);

  function resetSettings() {
    onEffort(model?.defaultReasoningEffort || supported[0] || "medium");
    onSpeed(normalizeModelSpeed(model, "standard"));
  }

  function selectModel(id: string) {
    const next = models.find((item) => item.model === id);
    onModel(id);
    if (
      next &&
      !next.supportedReasoningEfforts.some(
        (item) => item.reasoningEffort === effort,
      )
    ) {
      onEffort(
        next.defaultReasoningEffort ||
          next.supportedReasoningEfforts[0]?.reasoningEffort ||
          "medium",
      );
    }
    if (next) onSpeed(normalizeModelSpeed(next, speed));
    setPanel("main");
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      setPanel(null);
      setDragging(false);
    }
  }

  return (
    <div className="model-picker" ref={root} onKeyDown={handleKeyDown}>
      {panel && (
        <ModelControlPopover
          panel={panel}
          models={models}
          model={model}
          effort={effort}
          speed={speed}
          selectedEffort={selectedEffort}
          selectedModel={selectedModel}
          supported={supported}
          currentIndex={currentIndex}
          effortPercent={effortPercent}
          speedOptions={speedOptions}
          dragging={dragging}
          setDragging={setDragging}
          setPanel={setPanel}
          onEffort={onEffort}
          onSpeed={onSpeed}
          onReset={resetSettings}
          onSelectModel={selectModel}
        />
      )}
      <div className="model-trigger" aria-label="选择模型、速度和推理强度">
        <button
          type="button"
          className="model-trigger-speed"
          aria-label={`速度：${speedServiceLabel(speed)}`}
          aria-expanded={panel !== null}
          onClick={() =>
            setPanel((current) => (current === "main" ? null : "main"))
          }
        >
          <LightningIcon size={17} weight="fill" />
        </button>
        <button
          type="button"
          className="model-trigger-model"
          aria-label={`模型：${selectedModel}`}
          aria-expanded={panel !== null}
          onClick={() =>
            setPanel((current) => (current === "main" ? null : "main"))
          }
        >
          <span>{selectedModel}</span>
        </button>
        <button
          type="button"
          className="model-trigger-effort"
          aria-label={`推理强度：${selectedEffort}`}
          aria-expanded={panel !== null}
          onClick={() =>
            setPanel((current) => (current === "main" ? null : "main"))
          }
        >
          <span>{selectedEffort}</span>
          <CaretDownIcon size={14} />
        </button>
      </div>
    </div>
  );
}
