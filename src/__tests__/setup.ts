import "@testing-library/jest-dom/vitest";

if (
  typeof window !== "undefined" &&
  typeof window.localStorage?.clear !== "function"
) {
  const store = new Map<string, string>();
  const memoryStorage: Storage = {
    get length() {
      return store.size;
    },
    clear() {
      store.clear();
    },
    getItem(key: string) {
      return store.has(key) ? store.get(key)! : null;
    },
    key(index: number) {
      return Array.from(store.keys())[index] ?? null;
    },
    removeItem(key: string) {
      store.delete(key);
    },
    setItem(key: string, value: string) {
      store.set(key, String(value));
    },
  };
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: memoryStorage,
  });
}

if (
  typeof HTMLDialogElement !== "undefined" &&
  typeof HTMLDialogElement.prototype.showModal !== "function"
) {
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.removeAttribute("open");
      },
    },
  });
}
