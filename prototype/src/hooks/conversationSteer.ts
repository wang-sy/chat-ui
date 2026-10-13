import { bridge } from "../bridge.ts";

export type SteerResult = {
  status: "sent" | "busy" | "failed";
  error?: string;
};

export async function steerConversation(
  threadId: string,
  cwd: string,
  text: string,
  sync: () => Promise<unknown>,
): Promise<SteerResult> {
  try {
    await bridge.steer({
      threadId,
      cwd,
      text,
      requestId: crypto.randomUUID(),
    });
    try {
      await sync();
    } catch {
      // The event stream and regular poll will reconcile a delayed snapshot.
    }
    return { status: "sent" };
  } catch (cause) {
    const error = cause instanceof Error ? cause.message : String(cause);
    return /没有可接收|没有在运行|正在工作|工作中|working|active|busy/i.test(
      error,
    )
      ? { status: "busy" }
      : { status: "failed", error };
  }
}

export function steerIfActive(
  thread: { id: string; activeTurn: string | null } | undefined,
  cwd: string,
  text: string,
  sync: () => Promise<unknown>,
) {
  if (!thread?.activeTurn || thread.activeTurn === "starting") return null;
  return steerConversation(thread.id, cwd, text, sync);
}
