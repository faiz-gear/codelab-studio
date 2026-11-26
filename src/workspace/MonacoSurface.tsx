import { useEffect, useRef } from "react";
import * as monaco from "monaco-editor/editor/editor.api.js";
import "monaco-editor/editor/contrib/find/browser/findController.js";
import "monaco-editor/languages/definitions/go/register.js";
import "monaco-editor/languages/definitions/java/register.js";
import "monaco-editor/languages/definitions/javascript/register.js";
import "monaco-editor/languages/definitions/markdown/register.js";
import "monaco-editor/languages/definitions/python/register.js";
import "monaco-editor/languages/definitions/sql/register.js";
import "monaco-editor/languages/definitions/typescript/register.js";
import "monaco-editor/languages/definitions/yaml/register.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import type { GlobalSettings } from "../core/contracts";
import {
  MIN_SELECTION_CHARS,
  normalizeSelectionRect,
  type ActiveSelection,
} from "../tutor/contextRequest";
import type { LanguageSession } from "./languageSession";
import { connectLanguage } from "./monacoLanguage";
import { editorText, type EditorTab } from "./editorSession";

globalThis.MonacoEnvironment = { getWorker: () => new EditorWorker() };

monaco.editor.defineTheme("arc-light", {
  base: "vs",
  inherit: true,
  rules: [],
  colors: { "editor.background": "#FFFFFF", "editor.lineHighlightBackground": "#F7F7F7" },
});
monaco.editor.defineTheme("arc-dark", {
  base: "vs-dark",
  inherit: true,
  rules: [],
  colors: { "editor.background": "#141414", "editor.lineHighlightBackground": "#222222" },
});

const editorOptions = {
  automaticLayout: true,
  fontFamily: "JetBrains Mono, monospace",
  fontLigatures: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
} satisfies monaco.editor.IEditorOptions;

function language(path: string) {
  const basename = path.slice(path.lastIndexOf("/") + 1);
  const extension = basename.slice(basename.lastIndexOf("."));
  if (extension === ".pyi") return "python";
  return monaco.languages.getLanguages().find(({ extensions, filenames }) =>
    filenames?.includes(basename) || extensions?.includes(extension)
  )?.id;
}

type EditorModel = {
  model: monaco.editor.ITextModel;
  view: monaco.editor.ICodeEditorViewState | null;
  onChange: monaco.IDisposable;
  value: string;
};

export function MonacoTextSurface({
  tabs,
  path,
  label,
  settings,
  readOnly,
  languageSession,
  onEdit,
  onSelectionChange,
  onCursorLinesChange,
}: {
  tabs: EditorTab[];
  path: string | null;
  label: string;
  settings: GlobalSettings;
  readOnly: boolean;
  languageSession?: LanguageSession;
  onEdit: (path: string, text: string) => void;
  onSelectionChange?: (selection: ActiveSelection | null) => void;
  onCursorLinesChange?: (range: { startLine: number; endLine: number } | null) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const models = useRef(new Map<string, EditorModel>());
  const select = useRef<string | null>(null);
  const edit = useRef(onEdit);
  const selectionCb = useRef(onSelectionChange);
  const cursorCb = useRef(onCursorLinesChange);
  const pointerDown = useRef(false);
  const syncing = useRef(false);
  const languageBridge = useRef<ReturnType<typeof connectLanguage> | null>(null);

  useEffect(() => {
    edit.current = onEdit;
  }, [onEdit]);

  useEffect(() => {
    selectionCb.current = onSelectionChange;
  }, [onSelectionChange]);

  useEffect(() => {
    cursorCb.current = onCursorLinesChange;
  }, [onCursorLinesChange]);

  useEffect(() => {
    const node = host.current;
    if (!node) return;
    const instance = monaco.editor.create(node, { ...editorOptions, model: null });
    const modelsByPath = models.current;
    editor.current = instance;

    const emitSelection = () => {
      const sel = instance.getSelection();
      const model = instance.getModel();
      if (!sel || !model) {
        cursorCb.current?.(null);
        selectionCb.current?.(null);
        return;
      }
      cursorCb.current?.({
        startLine: sel.startLineNumber,
        endLine: sel.endLineNumber,
      });
      if (sel.isEmpty()) {
        selectionCb.current?.(null);
        return;
      }
      const text = model.getValueInRange(sel).trim();
      if (text.length < MIN_SELECTION_CHARS) {
        selectionCb.current?.(null);
        return;
      }
      const hostRect = node.getBoundingClientRect();
      const startPos = instance.getScrolledVisiblePosition(sel.getStartPosition());
      const endPos = instance.getScrolledVisiblePosition(sel.getEndPosition());
      const anchor = startPos ?? endPos;
      const rect = anchor
        ? {
            top: hostRect.top + (startPos ? startPos.top : anchor.top),
            bottom:
              hostRect.top +
              (endPos ? endPos.top + endPos.height : anchor.top + anchor.height),
            left: hostRect.left + anchor.left,
            right:
              hostRect.left +
              (startPos && endPos
                ? Math.max(startPos.left + 80, endPos.left)
                : anchor.left + 120),
          }
        : normalizeSelectionRect(null, node);
      selectionCb.current?.({
        zone: "editor",
        text,
        rect,
        filePath: select.current,
        startLine: sel.startLineNumber,
        endLine: sel.endLineNumber,
      });
    };

    const handleMouseDown = () => {
      pointerDown.current = true;
    };
    const handleMouseUp = () => {
      if (!pointerDown.current) return;
      pointerDown.current = false;
      emitSelection();
    };
    node.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mouseup", handleMouseUp);

    const onSel = instance.onDidChangeCursorSelection(() => {
      if (pointerDown.current) {
        const sel = instance.getSelection();
        if (sel) {
          cursorCb.current?.({
            startLine: sel.startLineNumber,
            endLine: sel.endLineNumber,
          });
        }
        return;
      }
      emitSelection();
    });
    const onScroll = instance.onDidScrollChange(() => {
      selectionCb.current?.(null);
    });

    return () => {
      node.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mouseup", handleMouseUp);
      onSel.dispose();
      onScroll.dispose();
      instance.dispose();
      editor.current = null;
      select.current = null;
      for (const { model, onChange } of modelsByPath.values()) {
        onChange.dispose();
        model.dispose();
      }
      modelsByPath.clear();
    };
  }, []);

  useEffect(() => {
    if (!languageSession) return;
    const bridge = connectLanguage(languageSession, () => {
      const path = select.current;
      const model = editor.current?.getModel();
      return path !== null && model ? { path, model } : undefined;
    });
    languageBridge.current = bridge;
    return () => {
      bridge.dispose();
      languageBridge.current = null;
    };
  }, [languageSession]);

  useEffect(() => {
    const instance = editor.current;
    if (!instance) return;
    const items = models.current;
    syncing.current = true;
    for (const tab of tabs) {
      const text = editorText(tab);
      const key = tab.file.path;
      if (text === null) continue;
      const item = items.get(key);
      if (!item) {
        const model = monaco.editor.createModel(text, language(key));
        const entry: EditorModel = {
          model, view: null, value: text,
          onChange: model.onDidChangeContent(() => {
            if (!syncing.current) {
              entry.value = model.getValue(monaco.editor.EndOfLinePreference.TextDefined, true);
              edit.current(key, entry.value);
            }
          }),
        };
        items.set(key, entry);
      } else if (text !== item.value) {
        item.value = text;
        item.model.setValue(text);
      }
    }

    const current = select.current === null ? undefined : items.get(select.current);
    const next = path === null ? undefined : items.get(path);
    if ((next?.model ?? null) !== instance.getModel()) {
      if (current) current.view = instance.saveViewState();
      instance.setModel(next?.model ?? null);
      if (next?.view) instance.restoreViewState(next.view);
      select.current = path;
      languageBridge.current?.refresh();
    }
    for (const [key, item] of items) {
      if (tabs.some((tab) => tab.file.path === key && editorText(tab) !== null)) continue;
      item.onChange.dispose();
      item.model.dispose();
      items.delete(key);
    }
    syncing.current = false;
  }, [path, tabs]);

  useEffect(() => {
    monaco.editor.setTheme(settings.theme === "dark" ? "arc-dark" : "arc-light");
    editor.current?.updateOptions({ ariaLabel: label, fontSize: settings.codeFontSize, readOnly });
  }, [label, readOnly, settings.codeFontSize, settings.theme]);

  return <div className="monaco-surface" ref={host} />;
}

export function MonacoDiffSurface({
  leftPath, rightPath, left, right, settings,
  leftLabel = leftPath, rightLabel = rightPath, showWhitespace = false, onReady,
}: {
  leftPath: string;
  rightPath: string;
  leftLabel?: string;
  rightLabel?: string;
  showWhitespace?: boolean;
  onReady?: (path: string) => void;
  left: string;
  right: string;
  settings: GlobalSettings;
}) {
  const host = useRef<HTMLDivElement>(null);
  const diff = useRef<monaco.editor.IStandaloneDiffEditor | null>(null);

  useEffect(() => {
    if (!host.current) return;
    const instance = monaco.editor.createDiffEditor(host.current, {
      ...editorOptions,
      readOnly: true,
      renderSideBySide: true,
      useInlineViewWhenSpaceIsLimited: false,
    });
    diff.current = instance;
    return () => {
      instance.dispose();
      diff.current = null;
    };
  }, []);

  useEffect(() => {
    const instance = diff.current;
    if (!instance) return;
    const original = monaco.editor.createModel(left, language(leftPath));
    const modified = monaco.editor.createModel(right, language(rightPath));
    let cancelled = false;
    const subscription = instance.onDidUpdateDiff(() => {
      if (instance.getModel()?.original === original && instance.getLineChanges() !== null) {
        onReady?.(leftPath);
      }
    });
    // StrictMode may clean up this setup before Monaco's worker can read its models.
    queueMicrotask(() => {
      if (!cancelled) instance.setModel({ original, modified });
    });
    return () => {
      cancelled = true;
      subscription.dispose();
      if (diff.current === instance) instance.setModel(null);
      original.dispose();
      modified.dispose();
    };
  }, [left, leftPath, onReady, right, rightPath]);

  useEffect(() => {
    monaco.editor.setTheme(settings.theme === "dark" ? "arc-dark" : "arc-light");
    diff.current?.getOriginalEditor().updateOptions({ ariaLabel: leftLabel, fontSize: settings.codeFontSize });
    diff.current?.getModifiedEditor().updateOptions({ ariaLabel: rightLabel, fontSize: settings.codeFontSize });
    diff.current?.updateOptions({ ignoreTrimWhitespace: !showWhitespace });
  }, [leftLabel, rightLabel, settings.codeFontSize, settings.theme, showWhitespace]);

  return <div className="monaco-surface" ref={host} />;
}
