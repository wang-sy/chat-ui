import { installCompositionRuntime } from "./bootstrap";
import { createEmbedSlot } from "./embedElement";
import { embedStyles } from "./embedStyles";
import { reportChildSize } from "./childSize";

export function compositionDocument(html: string, channel: string) {
  const document = new DOMParser().parseFromString(html, "text/html");
  const script = document.createElement("script");
  script.textContent = `(${installCompositionRuntime.toString()})(
    ${JSON.stringify(channel)}, ${createEmbedSlot.toString()},
    ${JSON.stringify(embedStyles)}, ${reportChildSize.toString()});`
    .replace(/<\/script/gi, "<\\/script");
  document.head.prepend(script);
  return `<!doctype html>\n${document.documentElement.outerHTML}`;
}
