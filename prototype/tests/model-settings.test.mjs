import assert from "node:assert/strict";
import test from "node:test";

import {
  normalizeModelSpeed,
  speedsForModel,
} from "../src/modelSettings.ts";

const efforts = [
  { reasoningEffort: "low", description: "" },
  { reasoningEffort: "medium", description: "" },
];

function model(overrides = {}) {
  return {
    model: "gpt-test",
    displayName: "GPT-Test",
    isDefault: false,
    defaultReasoningEffort: "medium",
    supportedReasoningEfforts: efforts,
    ...overrides,
  };
}

test("model speed capabilities hide unsupported ultrafast tiers", () => {
  const luna = model({
    additionalSpeedTiers: ["fast"],
    serviceTiers: [{ id: "priority", name: "Fast", description: "" }],
  });
  assert.deepEqual(
    speedsForModel(luna).map((item) => item.id),
    ["standard", "fast"],
  );
  assert.equal(normalizeModelSpeed(luna, "turbo"), "standard");
});

test("models with ultrafast expose the complete speed picker", () => {
  const sol = model({
    additionalSpeedTiers: ["fast", "ultrafast"],
    serviceTiers: [
      { id: "priority", name: "Fast", description: "" },
      { id: "ultrafast", name: "Ultrafast", description: "" },
    ],
  });
  assert.deepEqual(
    speedsForModel(sol).map((item) => item.id),
    ["standard", "fast", "turbo"],
  );
  assert.equal(normalizeModelSpeed(sol, "turbo"), "turbo");
});

test("legacy model payloads keep the permissive fallback", () => {
  assert.deepEqual(
    speedsForModel(model()).map((item) => item.id),
    ["standard", "fast", "turbo"],
  );
});
