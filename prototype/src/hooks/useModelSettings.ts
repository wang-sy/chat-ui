import { useEffect, useRef, useState } from "react";
import type { Chat, Model, PermissionMode } from "../bridge";
import {
  normalizeModelSpeed,
  normalizePermission,
  type ModelSpeed,
} from "../modelSettings";

export function useModelSettings(models: Model[], chat: Chat | undefined) {
  const [modelId, setModelId] = useState("");
  const [effort, setEffort] = useState("");
  const [speed, setSpeed] = useState<ModelSpeed>(() => {
    const stored = localStorage.getItem("chat-ui:speed");
    return stored === "fast" || stored === "turbo" ? stored : "standard";
  });
  const [permission, setPermission] = useState<PermissionMode>(() =>
    normalizePermission(localStorage.getItem("chat-ui:permission")),
  );
  const restored = useRef("");
  useEffect(() => {
    if (!chat) {
      restored.current = "";
      return;
    }
    if (restored.current === chat.id) return;
    restored.current = chat.id;
    setModelId(chat.model);
    setEffort(chat.effort);
    setSpeed(
      chat.speed === "fast" || chat.speed === "turbo" ? chat.speed : "standard",
    );
    setPermission(normalizePermission(chat.permission));
  }, [chat]);
  const model = models.find((item) => item.model === modelId);
  useEffect(() => {
    if (!models.length) return;
    const next = model || models.find((item) => item.isDefault) || models[0];
    if (!model) setModelId(next.model);
    if (
      !next.supportedReasoningEfforts.some(
        (item) => item.reasoningEffort === effort,
      )
    ) {
      setEffort(
        next.defaultReasoningEffort ||
          next.supportedReasoningEfforts[0]?.reasoningEffort ||
          "",
      );
    }
    const nextSpeed = normalizeModelSpeed(next, speed);
    if (nextSpeed !== speed) setSpeed(nextSpeed);
  }, [models, model, effort, speed]);
  useEffect(() => {
    localStorage.setItem("chat-ui:permission", permission);
  }, [permission]);
  useEffect(() => {
    localStorage.setItem("chat-ui:speed", speed);
  }, [speed]);
  return {
    model,
    modelId,
    setModelId,
    effort,
    setEffort,
    speed,
    setSpeed,
    permission,
    setPermission,
  };
}
