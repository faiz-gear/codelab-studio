const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(", ");

export function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute("hidden") && el.getAttribute("aria-hidden") !== "true"
  );
}

export function trapFocusOnTab(container: HTMLElement | null, event: KeyboardEvent): void {
  if (event.key !== "Tab" || !container) return;
  const focusables = getFocusableElements(container);
  if (focusables.length === 0) return;
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  const active = document.activeElement as HTMLElement | null;

  if (event.shiftKey) {
    if (!active || active === first || !container.contains(active)) {
      event.preventDefault();
      last.focus();
    }
  } else if (!active || active === last || !container.contains(active)) {
    event.preventDefault();
    first.focus();
  }
}

export function moveVerticalListFocus(
  container: HTMLElement | null,
  itemSelector: string,
  event: { key: string; preventDefault: () => void }
): void {
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key) || !container) {
    return;
  }
  const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector));
  if (items.length === 0) return;
  event.preventDefault();
  const currentIndex = items.indexOf(document.activeElement as HTMLElement);
  const nextIndex =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? items.length - 1
        : event.key === "ArrowDown"
          ? currentIndex >= 0
            ? (currentIndex + 1) % items.length
            : 0
          : currentIndex >= 0
            ? (currentIndex - 1 + items.length) % items.length
            : items.length - 1;
  items[nextIndex]?.focus();
}
