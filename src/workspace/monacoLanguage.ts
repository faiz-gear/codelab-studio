import * as monaco from "monaco-editor/editor/editor.api.js";
import "monaco-editor/editor/contrib/hover/browser/hoverContribution.js";
import "monaco-editor/editor/contrib/snippet/browser/snippetController2.js";
import "monaco-editor/editor/contrib/suggest/browser/suggestController.js";
import "monaco-editor/editor/contrib/gotoError/browser/gotoError.js";
import type { LanguageFeatureResult, LanguageMarkup, LanguageRange } from "../core/contracts";
import type { LanguageSession } from "./languageSession";

const { CompletionItemKind: Kind, CompletionItemInsertTextRule } = monaco.languages;
const completionKinds = [
  Kind.Text, Kind.Method, Kind.Function, Kind.Constructor, Kind.Field, Kind.Variable,
  Kind.Class, Kind.Interface, Kind.Module, Kind.Property, Kind.Unit, Kind.Value, Kind.Enum,
  Kind.Keyword, Kind.Snippet, Kind.Color, Kind.File, Kind.Reference, Kind.Folder,
  Kind.EnumMember, Kind.Constant, Kind.Struct, Kind.Event, Kind.Operator, Kind.TypeParameter,
];

function range(value: LanguageRange | null | undefined, model: monaco.editor.ITextModel) {
  if (!value) return undefined;
  const { start, end } = value;
  const first = new monaco.Position(start.line + 1, start.character + 1);
  const last = new monaco.Position(end.line + 1, end.character + 1);
  if (last.isBefore(first)) return undefined;
  return model.validateRange(monaco.Range.fromPositions(first, last));
}

function plaintext(value: string) {
  return value.replace(/[\\`*_{}[\]()<>#+.!|~>-]/g, "\\$&");
}

function documentation(content: LanguageMarkup, stringsAreMarkdown = false): monaco.IMarkdownString {
  if (typeof content === "string") return { value: stringsAreMarkdown ? content : plaintext(content) };
  if ("language" in content) {
    const language = content.language.replace(/[^a-zA-Z0-9_+-]/g, "");
    let longest = 2;
    for (const match of content.value.matchAll(/`+/g)) longest = Math.max(longest, match[0].length);
    const fence = "`".repeat(longest + 1);
    return { value: fence + language + "\n" + content.value + "\n" + fence };
  }
  return { value: content.kind === "plaintext" ? plaintext(content.value) : content.value };
}

function suggestions(
  result: Extract<LanguageFeatureResult, { kind: "completion" }>["value"],
  model: monaco.editor.ITextModel,
  cursor: monaco.Position,
): monaco.languages.CompletionList {
  const word = model.getWordUntilPosition(cursor);
  const fallback = new monaco.Range(cursor.lineNumber, word.startColumn, cursor.lineNumber, word.endColumn);
  return {
    incomplete: result.isIncomplete,
    suggestions: result.items.map((item) => {
      const edit = item.textEdit;
      const replace = range(edit?.range ?? edit?.replace, model);
      const insert = range(edit?.insert, model);
      const editRange = replace?.startLineNumber === cursor.lineNumber &&
        replace.endLineNumber === cursor.lineNumber && replace.containsPosition(cursor) ? replace : undefined;
      return {
        label: item.label,
        kind: completionKinds[(item.kind ?? 0) - 1] ?? Kind.Text,
        detail: item.detail ?? undefined,
        documentation: item.documentation == null ? undefined : documentation(item.documentation),
        filterText: item.filterText ?? undefined,
        sortText: item.sortText ?? undefined,
        insertText: edit?.newText ?? item.insertText ?? item.label,
        insertTextRules: item.insertTextFormat === 2 ? CompletionItemInsertTextRule.InsertAsSnippet : undefined,
        range: editRange && insert && insert.startLineNumber === cursor.lineNumber &&
          insert.endLineNumber === cursor.lineNumber && insert.containsPosition(cursor)
          ? { insert, replace: editRange } : editRange ?? fallback,
        additionalTextEdits: item.additionalTextEdits?.flatMap((extra) => {
          const target = range(extra.range, model);
          return target ? [{ range: target, text: extra.newText }] : [];
        }),
      };
    }),
  };
}

const markerSeverities = [
  monaco.MarkerSeverity.Error, monaco.MarkerSeverity.Warning,
  monaco.MarkerSeverity.Info, monaco.MarkerSeverity.Hint,
];

export function connectLanguage(
  session: LanguageSession,
  active: () => { path: string; model: monaco.editor.ITextModel } | undefined,
) {
  const selector = ["go", "java", "javascript", "typescript", "python"];
  const result = (feature: "completion" | "hover", model: monaco.editor.ITextModel,
    cursor: monaco.Position, token: monaco.CancellationToken) => {
    const current = active();
    return current?.model === model
      ? session.request(feature, current.path, cursor.lineNumber - 1, cursor.column - 1, () => token.isCancellationRequested)
      : null;
  };

  const completion = monaco.languages.registerCompletionItemProvider(selector, {
    triggerCharacters: ["."],
    async provideCompletionItems(model, cursor, _context, token) {
      const reply = await result("completion", model, cursor, token);
      return reply?.kind === "completion" ? suggestions(reply.value, model, cursor) : null;
    },
  });
  const hover = monaco.languages.registerHoverProvider(selector, {
    async provideHover(model, cursor, token) {
      const reply = await result("hover", model, cursor, token);
      if (reply?.kind !== "hover" || reply.value.contents.length === 0) return null;
      return {
        contents: reply.value.contents.map((content) => documentation(content, true)),
        range: range(reply.value.range, model),
      };
    },
  });

  let marked: monaco.editor.ITextModel | undefined;
  function refresh() {
    const current = active();
    if (marked && marked !== current?.model && !marked.isDisposed()) {
      monaco.editor.setModelMarkers(marked, "lsp", []);
    }
    marked = current?.model;
    if (!current) return;
    const { status, diagnostics } = session.snapshot();
    const model = current.model;
    const markers = status?.path === current.path ? diagnostics.flatMap((diagnostic): monaco.editor.IMarkerData[] => {
      const target = range(diagnostic.range, model);
      if (!target) return [];
      return [{
        ...target,
        message: diagnostic.message,
        source: diagnostic.source ?? undefined,
        code: diagnostic.code == null ? undefined : String(diagnostic.code),
        severity: markerSeverities[(diagnostic.severity ?? 0) - 1] ?? monaco.MarkerSeverity.Error,
      }];
    }) : [];
    monaco.editor.setModelMarkers(model, "lsp", markers);
  }

  const unsubscribe = session.subscribe(refresh);
  refresh();
  return {
    refresh,
    dispose() {
      unsubscribe();
      hover.dispose();
      completion.dispose();
      if (marked && !marked.isDisposed()) monaco.editor.setModelMarkers(marked, "lsp", []);
    },
  };
}
