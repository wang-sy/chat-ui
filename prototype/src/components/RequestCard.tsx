import { useState } from "react";
import { CodeIcon } from "@phosphor-icons/react";
import type { AgentRequest } from "../bridge";

type Props = {
  request: AgentRequest;
  onReply: (
    decision: string,
    answers?: Record<string, { answers: string[] }>,
    content?: Record<string, unknown> | null,
  ) => Promise<void>;
};

export function RequestCard({ request, onReply }: Props) {
  const [pending, setPending] = useState(false);
  const [answers, setAnswers] = useState<Record<string, { answers: string[] }>>(
    {},
  );
  const [error, setError] = useState("");
  const elicitation = request.method === "mcpServer/elicitation/request";
  const approval = request.method.includes("Approval");
  const schemaProperties = request.params.requestedSchema?.properties || {};
  const required = new Set(request.params.requestedSchema?.required || []);
  const fields = [
    ...(request.params.questions || []).map((question) => ({
      ...question,
      required: true,
      type: "string",
    })),
    ...Object.entries(schemaProperties).map(([id, schema]) => ({
      id,
      question: schema.title || id,
      isSecret: schema.format === "password",
      options: schema.enum?.map((label) => ({ label })),
      required: required.has(id),
      type: schema.type || "string",
    })),
  ];
  const mcpContent = () =>
    Object.fromEntries(
      fields
        .filter((field) => answers[field.id]?.answers[0] !== undefined)
        .map((field) => {
          const value = answers[field.id]?.answers[0] || "";
          if (field.type === "boolean") return [field.id, value === "true"];
          if (field.type === "number" || field.type === "integer")
            return [field.id, Number(value)];
          return [field.id, value];
        }),
    );

  async function respond(
    decision: string,
    nextAnswers?: Record<string, { answers: string[] }>,
    content?: Record<string, unknown> | null,
  ) {
    setPending(true);
    setError("");
    try {
      await onReply(decision, nextAnswers, content);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPending(false);
    }
  }

  return (
    <section className="approval-card">
      <div className="approval-card-heading">
        <span className="approval-icon">
          <CodeIcon size={17} />
        </span>
        <div>
          <strong>{approval ? "需要你的确认" : "需要你的输入"}</strong>
          <small>
            {request.params.reason ||
              request.params.message ||
              "Codex 正在等待这个操作继续。"}
          </small>
        </div>
      </div>
      {request.params.command && (
        <code className="approval-command">
          {Array.isArray(request.params.command)
            ? request.params.command.join(" ")
            : request.params.command}
        </code>
      )}
      {request.params.grantRoot && (
        <span className="approval-path">{request.params.grantRoot}</span>
      )}
      {request.params.url && (
        <a
          className="approval-path"
          href={request.params.url}
          target="_blank"
          rel="noreferrer"
        >
          打开授权链接
        </a>
      )}
      {fields.map((field) => (
        <label className="approval-question" key={field.id}>
          <span>{field.question}</span>
          <input
            type={field.isSecret ? "password" : "text"}
            value={answers[field.id]?.answers[0] || ""}
            onChange={(event) =>
              setAnswers((current) => ({
                ...current,
                [field.id]: { answers: [event.target.value] },
              }))
            }
            placeholder={field.options
              ?.map((option) => option.label)
              .join(" / ")}
          />
        </label>
      ))}
      <div className="approval-actions">
        <button
          type="button"
          disabled={pending}
          onClick={() => void respond("decline")}
        >
          {approval ? "拒绝" : "取消"}
        </button>
        <button
          type="button"
          className="approval-primary"
          disabled={
            pending ||
            fields.some(
              (field) =>
                field.required && !(answers[field.id]?.answers[0] || "").trim(),
            )
          }
          onClick={() =>
            void respond(
              "accept",
              approval || !elicitation ? answers : undefined,
              elicitation
                ? request.params.url
                  ? null
                  : mcpContent()
                : undefined,
            )
          }
        >
          {approval ? "允许本次" : request.params.url ? "已打开" : "提交"}
        </button>
      </div>
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
