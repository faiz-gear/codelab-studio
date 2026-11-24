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

export function MonacoTextSurface({ tabs, path, label, settings, readOnly, onEdit }: {
  tabs: EditorTab[];
  path: string | null;
  label: string;
  settings: GlobalSettings;
  readOnly: boolean;
  onEdit: (path: string, text: string) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const models = useRef(new Map<string, EditorModel>());
  const select = useRef<string | null>(null);
  const edit = useRef(onEdit);
  const syncing = useRef(false);

  useEffect(() => {
    edit.current = onEdit;
  }, [onEdit]);

  useEffect(() => {
    if (!host.current) return;
    const instance = monaco.editor.create(host.current, { ...editorOptions, model: null });
    const modelsByPath = models.current;
    editor.current = instance;
    return () => {
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

export function MonacoDiffSurface({ leftPath, rightPath, left, right, settings }: {
  leftPath: string;
  rightPath: string;
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
    instance.setModel({ original, modified });
    return () => {
      if (diff.current === instance) instance.setModel(null);
      original.dispose();
      modified.dispose();
    };
  }, [left, leftPath, right, rightPath]);

  useEffect(() => {
    monaco.editor.setTheme(settings.theme === "dark" ? "arc-dark" : "arc-light");
    diff.current?.getOriginalEditor().updateOptions({ ariaLabel: leftPath, fontSize: settings.codeFontSize });
    diff.current?.getModifiedEditor().updateOptions({ ariaLabel: rightPath, fontSize: settings.codeFontSize });
  }, [leftPath, rightPath, settings.codeFontSize, settings.theme]);

  return <div className="monaco-surface" ref={host} />;
}
