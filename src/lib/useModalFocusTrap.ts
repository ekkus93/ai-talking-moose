import { useEffect } from "react";

type ElementRef = {
  readonly current: HTMLElement | null;
};

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(",");

const focusableElements = (container: HTMLElement) =>
  Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter((element) => element.getAttribute("aria-hidden") !== "true");

export const useModalFocusTrap = (
  active: boolean,
  containerRef: ElementRef,
  initialFocusRef?: ElementRef,
) => {
  useEffect(() => {
    if (!active) return;

    const container = containerRef.current;
    if (!container) return;

    const previouslyFocused =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;

    const focusInitialTarget = () => {
      const initialTarget = initialFocusRef?.current;
      if (initialTarget && container.contains(initialTarget)) {
        initialTarget.focus();
        return;
      }

      const firstFocusable = focusableElements(container)[0];
      (firstFocusable ?? container).focus();
    };

    focusInitialTarget();

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;

      const focusable = focusableElements(container);
      if (focusable.length === 0) {
        event.preventDefault();
        container.focus();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const current = document.activeElement;
      const focusOutside =
        !(current instanceof Node) || !container.contains(current);

      if (event.shiftKey) {
        if (current === first || focusOutside) {
          event.preventDefault();
          last.focus();
        }
        return;
      }

      if (current === last || focusOutside) {
        event.preventDefault();
        first.focus();
      }
    };

    const handleFocusIn = (event: FocusEvent) => {
      const target = event.target;
      if (target instanceof Node && !container.contains(target)) {
        focusInitialTarget();
      }
    };

    document.addEventListener("keydown", handleKeyDown, true);
    document.addEventListener("focusin", handleFocusIn, true);

    return () => {
      document.removeEventListener("keydown", handleKeyDown, true);
      document.removeEventListener("focusin", handleFocusIn, true);
      if (previouslyFocused?.isConnected) {
        previouslyFocused.focus();
      }
    };
  }, [active, containerRef, initialFocusRef]);
};
