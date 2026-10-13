import type { Model, PermissionMode } from "./bridge";

export const efforts = ["low", "medium", "high", "xhigh", "max", "ultra"];
export const effortNames: Record<string, string> = {
  low: "低",
  medium: "标准",
  high: "深度",
  xhigh: "极深",
  max: "Max",
  ultra: "Ultra",
};
export const effortDescriptions: Record<string, string> = {
  low: "快速响应，使用较轻的推理",
  medium: "平衡速度与推理深度",
  high: "为复杂问题提供更深推理",
  xhigh: "为复杂问题提供额外深度",
  max: "最深推理，适合最难的问题",
  ultra: "最大推理，并自动分派任务",
};

export const speeds = [
  { id: "standard", title: "标准", detail: "默认速度" },
  { id: "fast", title: "快速", detail: "1.5 倍速度，用量更多" },
  {
    id: "turbo",
    title: "超快",
    detail: "The fastest available responses for latency-sensitive work.",
  },
] as const;
export type ModelSpeed = (typeof speeds)[number]["id"];
export type SpeedOption = (typeof speeds)[number];

type SpeedModel = Pick<Model, "serviceTiers" | "additionalSpeedTiers">;

/**
 * The model catalog reports speed capabilities in two compatible shapes:
 * `additionalSpeedTiers` uses product names (`fast`, `ultrafast`) while
 * `serviceTiers` uses Codex service ids (`priority`, `ultrafast`). Keep the
 * UI ids stable and translate both forms here so every picker shares one
 * source of truth.
 */
export function speedsForModel(model?: SpeedModel): readonly SpeedOption[] {
  const additional = model?.additionalSpeedTiers;
  const serviceTiers = model?.serviceTiers;
  const hasCatalog = Array.isArray(additional) || Array.isArray(serviceTiers);
  if (!hasCatalog) return speeds;

  const capabilities = new Set<string>();
  for (const value of additional ?? []) capabilities.add(value.toLowerCase());
  for (const tier of serviceTiers ?? []) {
    capabilities.add(tier.id.toLowerCase());
    capabilities.add(tier.name.toLowerCase());
  }

  const hasFast = capabilities.has("fast") || capabilities.has("priority");
  const hasTurbo =
    capabilities.has("turbo") || capabilities.has("ultrafast");
  return speeds.filter(
    (item) =>
      item.id === "standard" ||
      (item.id === "fast" && hasFast) ||
      (item.id === "turbo" && hasTurbo),
  );
}

export function normalizeModelSpeed(
  model: SpeedModel | undefined,
  value: ModelSpeed,
): ModelSpeed {
  const available = speedsForModel(model);
  return available.some((item) => item.id === value)
    ? value
    : (available[0]?.id ?? "standard");
}

export function speedServiceLabel(speed: ModelSpeed) {
  return speeds.find((item) => item.id === speed)?.title || speeds[0].title;
}

export const modelName = (name: string) =>
  name.replace(/-/g, " ").replace(/^GPT /, "GPT-");

export function normalizePermission(
  value: string | null | undefined,
): PermissionMode {
  return value === "auto" || value === "full" ? value : "ask";
}
