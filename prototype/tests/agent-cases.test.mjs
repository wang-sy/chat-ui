import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const root = path.join(import.meta.dirname, "..");

async function source(relativePath) {
  return readFile(path.join(root, relativePath), "utf8");
}

test("Main/View agent regression fixture contains ten ownership cases", async () => {
  const cases = JSON.parse(await source("agent-cases.json"));
  assert.equal(cases.length, 10);
  assert.deepEqual(
    cases.map((item) => item.id),
    Array.from({ length: 10 }, (_, index) => `C${String(index + 1).padStart(2, "0")}`),
  );
  for (const item of cases) {
    assert.equal(typeof item.kind, "string");
    assert.equal(typeof item.main, "string");
    assert.equal(typeof item.view, "string");
    assert.equal(typeof item.invariant, "string");
  }
});

test("the ten cases are backed by executable ownership contracts", async () => {
  const [cases, spec, protocol, orchestration, viewRuns, events, plans, queue, steer, eventsRoute, pageSurface, requests, threads, persistence, executionStatus, conversationPanel] =
    await Promise.all([
      source("agent-cases.json").then(JSON.parse),
      source("../AGENT_CASES.md"),
      Promise.all([
        source("src-tauri/src/runtime/chat_service/protocol.rs"),
        source("src-tauri/src/runtime/chat_service/prompts/main.md"),
        source("src-tauri/src/runtime/chat_service/prompts/view.md"),
      ]).then((parts) => parts.join("\n")),
      source("src-tauri/src/runtime/chat_service/orchestration.rs"),
      source("src-tauri/src/runtime/chat_service/view_runs.rs"),
      source("src-tauri/src/runtime/chat_service/events.rs"),
      source("src-tauri/src/space/plans.rs"),
      source("src/conversationQueue.ts"),
      source("src/hooks/conversationSteer.ts"),
      source("src/hooks/conversationEvents.ts"),
      source("src/components/MainPageSurface.tsx"),
      source("src-tauri/src/runtime/chat_service/requests.rs"),
      source("src-tauri/src/runtime/chat_service/threads.rs"),
      source("src-tauri/src/space/persistence.rs"),
      source("src/components/ExecutionStatus.tsx"),
      source("src/components/ConversationPanel.tsx"),
    ]);
  assert.match(spec, /统一边界/);
  const contracts = {
    C01: [[protocol, /plan_from_payload/], [protocol, /plan:\s*null/]],
    C02: [[protocol, /MAIN_THREAD_INSTRUCTIONS/], [orchestration, /start_view_thread/], [viewRuns, /view_thread_instructions/]],
    C03: [[protocol, /Reuse its id\/key/], [viewRuns, /Updated contract/]],
    C04: [[protocol, /postMessage/], [eventsRoute, /routeViewEvent/], [pageSurface, /contentWindow/]],
    C05: [[orchestration, /turn\/interrupt/], [plans, /remove_views/]],
    C06: [[viewRuns, /set_view_status/], [events, /turn\/completed/], [executionStatus, /RunningStatus/], [executionStatus, /currentActivityText/], [conversationPanel, /ConversationHistory/]],
    C07: [[queue, /MAX_QUEUE_ITEMS/], [steer, /steerConversation/], [orchestration, /turn\/interrupt/]],
    C08: [[plans, /status == "ready" && html\.trim\(\)\.is_empty\(\)/], [orchestration, /update_view_page/]],
    C09: [[requests, /validated_decision/], [requests, /item\/tool\/requestUserInput/]],
    C10: [[threads, /thread\/resume/], [persistence, /recover_after_restart/]],
  };
  assert.deepEqual(cases.map((item) => item.id), Object.keys(contracts));
  for (const [id, assertions] of Object.entries(contracts)) {
    for (const [text, pattern] of assertions) {
      assert.match(text, pattern, `${id} contract is no longer wired`);
    }
  }
});

test("the ten cases have runnable behavior coverage", async () => {
  const [behavior, spaceTests, protocolTests, eventTests, requestTests] =
    await Promise.all([
      source("tests/agent-cases.behavior.test.mjs"),
      source("src-tauri/src/space/tests.rs"),
      source("src-tauri/src/runtime/chat_service/protocol.rs"),
      source("src-tauri/src/runtime/chat_service/events_tests.rs"),
      source("src-tauri/src/runtime/chat_service/requests.rs"),
    ]);
  for (const id of ["C04", "C07"]) {
    assert.match(behavior, new RegExp(`${id}:`));
  }
  for (const [id, text] of [
    ["C01", protocolTests],
    ["C02", spaceTests],
    ["C03", spaceTests],
    ["C05", spaceTests],
    ["C06", eventTests],
    ["C08", spaceTests],
    ["C09", `${eventTests}\n${requestTests}`],
    ["C10", spaceTests],
  ]) {
    assert.match(text, new RegExp(`case_${id.toLowerCase()}_`), `${id} behavior test is missing`);
  }
});
