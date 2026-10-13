import type { AppState } from "./bridge.ts";

/** Polls can overtake queued SSE frames. Reject the older whole-workspace
 * snapshot so it cannot erase a newly created View's visible conversation. */
export function newestState(current: AppState, incoming: AppState, event = false): AppState {
  if (current.instanceId && incoming.instanceId === current.instanceId &&
      typeof current.revision === "number" && typeof incoming.revision === "number") {
    if (incoming.revision < current.revision || (event && incoming.revision === current.revision)) return current;
  }
  return incoming;
}
