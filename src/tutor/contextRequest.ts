import type {
  CheckResult,
  ContextQuote,
  CourseCheckSpec,
  Locale,
} from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";

export const MIN_SELECTION_CHARS = 2;

export type SelectionZone = "manual" | "editor" | "terminal";

export interface SelectionRect {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface ActiveSelection {
  zone: SelectionZone;
  text: string;
  rect: SelectionRect;
  filePath?: string | null;
  startLine?: number | null;
  endLine?: number | null;
}

export interface EditorCursorContext {
  filePath: string | null;
  startLine: number | null;
  endLine: number | null;
}

export type SelectionQuickAction =
  | "explainConcept"
  | "explainCode"
  | "checkIssues"
  | "diagnoseError"
  | "ask";

export interface TutorAskRequest {
  id: number;
  quote: ContextQuote;
  initialPrompt: string;
}

export function quickPromptForSelectionAction(
  action: SelectionQuickAction,
  messages: Messages
): string {
  if (action === "ask") return "";
  return messages.tutor.quickPrompts[action];
}

export function htmlToPlainText(html: string): string {
  if (typeof DOMParser !== "undefined") {
    const doc = new DOMParser().parseFromString(html, "text/html");
    return (doc.body.textContent ?? "").replace(/\s+/g, " ").trim();
  }
  return html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
}

export function formatCheckFailureQuote(
  check: CourseCheckSpec | undefined,
  result: CheckResult,
  messages: Messages,
  locale: Locale
): string {
  const title = check?.title ?? result.checkId;
  const lines = [
    `[${messages.verification.status[result.status]}] ${title} (${result.durationMs} ms)`,
  ];
  if (result.error) {
    lines.push(formatDomainError(result.error, locale));
  }
  for (const diff of result.diffs) {
    lines.push(
      `${diff.label}: ${messages.verification.expected} ${diff.expected} | ${messages.verification.actual} ${diff.actual}`
    );
  }
  for (const testCase of result.testCases) {
    if (testCase.status !== "passed") {
      lines.push(
        `${testCase.name}: ${testCase.output || messages.verification.status[testCase.status]}`
      );
    }
  }
  if (lines.length === 1 && result.rawOutput.trim()) {
    lines.push(result.rawOutput.trim().slice(0, 600));
  }
  return lines.join("\n");
}

const PORT_CONFLICT_QUOTE_MAX_LINES = 6;
const PORT_CONFLICT_QUOTE_MAX_CHARS = 600;

/**
 * Extracts a concise port-conflict excerpt for the pinned Quote Card so we do
 * not duplicate the full 200-line TerminalErrors Context Snapshot chip.
 */
export function extractPortConflictExcerpt(
  recentErrors: string[],
  fallbackText: string
): string {
  const cleaned = recentErrors.map((line) => line.trim()).filter(Boolean);
  if (cleaned.length === 0) return fallbackText;
  const conflictLines = cleaned.filter((line) => {
    const lower = line.toLowerCase();
    return (
      lower.includes("address already in use") ||
      lower.includes("eaddrinuse") ||
      lower.includes("only one usage of each socket address") ||
      lower.includes("port is already allocated")
    );
  });
  const selected = (
    conflictLines.length > 0 ? conflictLines : cleaned
  ).slice(-PORT_CONFLICT_QUOTE_MAX_LINES);
  const joined = selected.join("\n");
  return joined.length > PORT_CONFLICT_QUOTE_MAX_CHARS
    ? joined.slice(joined.length - PORT_CONFLICT_QUOTE_MAX_CHARS)
    : joined;
}

export function normalizeSelectionRect(
  rect: DOMRect | SelectionRect | null | undefined,
  fallbackEl?: HTMLElement | null
): SelectionRect {
  if (rect && (rect.bottom > rect.top || rect.right > rect.left)) {
    return {
      top: rect.top,
      bottom: rect.bottom,
      left: rect.left,
      right: rect.right,
    };
  }
  const elRect = fallbackEl?.getBoundingClientRect();
  if (elRect && (elRect.width > 0 || elRect.height > 0)) {
    return {
      top: elRect.top + 16,
      bottom: elRect.top + 38,
      left: elRect.left + 16,
      right: Math.min(elRect.right - 16, elRect.left + 220),
    };
  }
  return { top: 180, bottom: 204, left: 120, right: 280 };
}

export function extractDomSelectionWithin(
  container: HTMLElement | null,
  zone: SelectionZone,
  excludeInteractive = false
): ActiveSelection | null {
  if (!container || typeof window === "undefined") return null;
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return null;
  const anchorNode = selection.anchorNode;
  const focusNode = selection.focusNode;
  if (!anchorNode || !focusNode) return null;
  if (!container.contains(anchorNode) || !container.contains(focusNode)) return null;

  if (excludeInteractive) {
    const anchorEl =
      anchorNode.nodeType === Node.ELEMENT_NODE
        ? (anchorNode as Element)
        : anchorNode.parentElement;
    if (anchorEl?.closest("button, input, textarea, select, [data-selection-ignore]")) {
      return null;
    }
  }

  const text = selection.toString().trim();
  if (text.length < MIN_SELECTION_CHARS) return null;

  const range = selection.getRangeAt(0);
  const rawRect =
    typeof range.getBoundingClientRect === "function" ? range.getBoundingClientRect() : null;
  const rect = normalizeSelectionRect(rawRect, container);
  return { zone, text, rect };
}
