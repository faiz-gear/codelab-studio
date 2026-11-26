import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef } from "react";

import type { GlobalSettings } from "../core/contracts";
import {
  MIN_SELECTION_CHARS,
  normalizeSelectionRect,
  type ActiveSelection,
} from "../tutor/contextRequest";
import type { CommandWorkbench } from "./CommandWorkbench";

export default function TerminalViewport({
  workbench,
  id,
  settings,
  clearCount,
  label,
  onError,
  onSelectionChange,
}: {
  workbench: CommandWorkbench;
  id: number;
  settings: GlobalSettings;
  clearCount: number;
  label: string;
  onError: (error: unknown) => void;
  onSelectionChange?: (selection: ActiveSelection | null) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const cursor = useRef(workbench.startCursor(id));
  const handledClear = useRef(clearCount);
  const selectionCb = useRef(onSelectionChange);
  const pointerDown = useRef(false);
  const lastPointer = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    selectionCb.current = onSelectionChange;
  }, [onSelectionChange]);

  useEffect(() => {
    const node = host.current;
    if (!node) return;
    const emulator = new Terminal({
      scrollback: 5_000,
      fontFamily: '"JetBrains Mono", monospace',
      fontSize: 12.5,
      lineHeight: 1.6,
      minimumContrastRatio: 4.5,
    });
    terminal.current = emulator;
    const fit = new FitAddon();
    emulator.loadAddon(fit);
    emulator.open(node);

    const onData = emulator.onData((text) => {
      // Tauri async invocations may complete out of order. Send ordered batches.
      queuedInput += text;
      if (inputFrame === undefined) inputFrame = requestAnimationFrame(flushInput);
    });
    let queuedInput = "";
    let inputFrame: number | undefined;
    let writing = false;
    let alive = true;
    async function flushInput() {
      inputFrame = undefined;
      if (writing || !alive) return;
      writing = true;
      try {
        while (queuedInput && alive) {
          let count = Math.min(16_384, queuedInput.length);
          // Avoid splitting a UTF-16 surrogate pair in a pasted string.
          if (/[\uD800-\uDBFF]/.test(queuedInput[count - 1])) count--;
          const text = queuedInput.slice(0, count);
          queuedInput = queuedInput.slice(count);
          await workbench.updateTerminal({ type: "writeTerminal", payload: { ...workbench.target(id), text } });
        }
      } catch (error) {
        if (alive) onError(error);
      } finally {
        writing = false;
      }
    }
    const onResize = emulator.onResize(({ cols, rows }) => {
      void workbench.updateTerminal({ type: "resizeTerminal", payload: { ...workbench.target(id), cols, rows } })
        .catch((error: unknown) => { if (alive) onError(error); });
    });

    const emitSelection = () => {
      const text = emulator.getSelection().trim();
      if (text.length < MIN_SELECTION_CHARS) {
        selectionCb.current?.(null);
        return;
      }
      const screenEl = node.querySelector<HTMLElement>(".xterm-screen") ?? node;
      const screenRect = screenEl.getBoundingClientRect();
      const pos = emulator.getSelectionPosition();
      let rect = normalizeSelectionRect(null, screenEl);
      if (pos && screenRect.height > 0 && screenRect.width > 0) {
        const cellH = screenRect.height / Math.max(1, emulator.rows);
        const cellW = screenRect.width / Math.max(1, emulator.cols);
        const viewportY = emulator.buffer.active.viewportY;
        const startRow = Math.max(0, Math.min(emulator.rows - 1, pos.start.y - viewportY));
        const endRow = Math.max(startRow, Math.min(emulator.rows - 1, pos.end.y - viewportY));
        rect = {
          top: screenRect.top + startRow * cellH,
          bottom: screenRect.top + (endRow + 1) * cellH,
          left: screenRect.left + pos.start.x * cellW,
          right: screenRect.left + Math.max(pos.start.x + 6, pos.end.x) * cellW,
        };
      } else if (lastPointer.current) {
        rect = {
          top: lastPointer.current.y - 16,
          bottom: lastPointer.current.y + 8,
          left: lastPointer.current.x - 60,
          right: lastPointer.current.x + 60,
        };
      }
      selectionCb.current?.({ zone: "terminal", text, rect });
    };

    const handleMouseDown = () => {
      pointerDown.current = true;
    };
    const handleMouseUp = (event: MouseEvent) => {
      if (!pointerDown.current) return;
      pointerDown.current = false;
      lastPointer.current = { x: event.clientX, y: event.clientY };
      emitSelection();
    };
    node.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mouseup", handleMouseUp);

    const onSelection = emulator.onSelectionChange(() => {
      if (!pointerDown.current) emitSelection();
    });
    const onScroll = emulator.onScroll(() => {
      selectionCb.current?.(null);
    });
    const observer = new ResizeObserver(() => fit.fit());
    observer.observe(node);
    fit.fit();

    let frame: number | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let activeUntil = 0;
    async function poll() {
      try {
        const { output } = await workbench.read(id, cursor.current);
        if (!alive || !output) return;
        if (output.reset) emulator.reset();
        if (output.data) {
          const bytes = Uint8Array.from(atob(output.data), (character) => character.charCodeAt(0));
          await new Promise<void>((resolve) => emulator.write(bytes, resolve));
          activeUntil = performance.now() + 300;
        }
        if (!alive) return;
        cursor.current = output.cursor;
        if (output.more || performance.now() < activeUntil) frame = requestAnimationFrame(() => void poll());
        else timer = setTimeout(() => void poll(), 70);
      } catch (error) {
        if (alive) onError(error);
      }
    }
    void poll();
    return () => {
      alive = false;
      if (frame !== undefined) cancelAnimationFrame(frame);
      if (inputFrame !== undefined) cancelAnimationFrame(inputFrame);
      if (timer !== undefined) clearTimeout(timer);
      observer.disconnect();
      node.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mouseup", handleMouseUp);
      onSelection.dispose();
      onScroll.dispose();
      onData.dispose();
      onResize.dispose();
      emulator.dispose();
      terminal.current = null;
      selectionCb.current?.(null);
    };
  }, [id, onError, workbench]);

  useEffect(() => {
    if (clearCount === handledClear.current) return;
    handledClear.current = clearCount;
    workbench.clearAt(id, cursor.current);
    terminal.current?.clear();
  }, [clearCount, id, workbench]);

  const theme = settings.theme;
  useEffect(() => {
    const surface = host.current;
    const emulator = terminal.current;
    if (!surface || !emulator) return;
    const computed = getComputedStyle(surface);
    const foreground = computed.color;
    emulator.options.theme = { foreground, cursor: foreground, background: computed.backgroundColor };
  }, [theme]);

  return <div ref={host} className="terminal-viewport" role="region" aria-label={label} />;
}
