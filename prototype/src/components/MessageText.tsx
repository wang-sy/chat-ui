import { Fragment, type ReactNode } from "react";

// A small, text-only Markdown renderer: model HTML is never executed in chat.
function inline(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\(https?:\/\/[^\s)]+\))/g).map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**")) return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith("`") && part.endsWith("`")) return <code key={index}>{part.slice(1, -1)}</code>;
    const link = part.match(/^\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)$/);
    if (link) return <a key={index} href={link[2]} target="_blank" rel="noreferrer">{link[1]}</a>;
    return <Fragment key={index}>{part}</Fragment>;
  });
}

export function MessageText({ text }: { text: string }) {
  const blocks: ReactNode[] = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith("```")) {
      const key = i;
      const code: string[] = [];
      while (++i < lines.length && !lines[i].startsWith("```")) code.push(lines[i]);
      blocks.push(<pre key={key}><code>{code.join("\n")}</code></pre>);
    } else if (/^#{1,6}\s/.test(line)) {
      blocks.push(<p className="message-subheading" key={i}>{inline(line.replace(/^#{1,6}\s+/, ""))}</p>);
    } else if (/^\s*([-*]|\d+[.)])\s+/.test(line)) {
      blocks.push(<div className="message-list-line" key={i}>{inline(line.replace(/^\s*[-*]\s+/, "• "))}</div>);
    } else if (line.trim()) {
      blocks.push(<p key={i}>{inline(line)}</p>);
    } else {
      blocks.push(<div className="message-paragraph-gap" key={i} />);
    }
  }
  return <div className="message-text">{blocks}</div>;
}
