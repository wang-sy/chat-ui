/** Injected into the isolated child document; reports size without sharing its DOM. */
export function reportChildSize() {
  let previous = 0;
  let scheduled = false;
  const measure = () => {
    scheduled = false;
    const body = document.body;
    if (!body || !body.getBoundingClientRect().width) return;
    const style = getComputedStyle(body);
    const height = Math.ceil(Math.max(body.scrollHeight, body.getBoundingClientRect().height)
      + (parseFloat(style.marginTop) || 0) + (parseFloat(style.marginBottom) || 0));
    if (height > 0 && Math.abs(height - previous) > 1) {
      previous = height;
      parent.postMessage({ type: "chat-ui:embed-size", height }, "*");
    }
  };
  const schedule = () => {
    if (!scheduled) { scheduled = true; requestAnimationFrame(measure); }
  };
  const start = () => {
    new ResizeObserver(schedule).observe(document.body);
    new MutationObserver(schedule).observe(document.body, { childList: true, subtree: true, attributes: true });
    document.addEventListener("load", schedule, true);
    document.fonts?.ready.then(schedule);
    schedule();
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start, { once: true });
  else start();
}
