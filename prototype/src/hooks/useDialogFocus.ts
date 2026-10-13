import { useEffect, type RefObject } from "react";

export function useDialogFocus(ref: RefObject<HTMLElement | null>, open: boolean) {
  useEffect(() => {
    const dialog = ref.current;
    if (!open || !dialog) return;
    const previous = document.activeElement as HTMLElement | null;
    const frame = requestAnimationFrame(() => {
      (dialog.querySelector<HTMLElement>("[data-initial-focus]") || dialog).focus();
    });
    function containFocus(event: KeyboardEvent) {
      if (event.key !== "Tab" || (event.target as Element).closest('[role="dialog"]') !== dialog) return;
      const controls = [...dialog!.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), [tabindex="0"]',
      )].filter((element) => !element.closest("[inert]") && element.getClientRects().length);
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (!first) { event.preventDefault(); return; }
      if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === dialog)) {
        event.preventDefault();
        first.focus();
      }
    }
    dialog.addEventListener("keydown", containFocus);
    return () => {
      cancelAnimationFrame(frame);
      dialog.removeEventListener("keydown", containFocus);
      if (previous?.isConnected) previous.focus();
    };
  }, [open, ref]);
}
