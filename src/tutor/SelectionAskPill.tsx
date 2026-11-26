import { Fragment, useEffect, useRef } from "react";
import { SparklesIcon } from "../components/Icons";
import type { Messages } from "../i18n";
import type { ActiveSelection, SelectionQuickAction, SelectionZone } from "./contextRequest";

const PILL_HEIGHT = 34;
const PILL_GAP = 8;
const TOPBAR_SAFE_TOP = 54;
const VIEWPORT_MARGIN = 10;

interface PillActionItem {
  id: SelectionQuickAction;
  label: string;
}

function actionsForZone(zone: SelectionZone, messages: Messages): PillActionItem[] {
  const s = messages.tutor.selection;
  switch (zone) {
    case "manual":
      return [
        { id: "explainConcept", label: s.explainConcept },
        { id: "ask", label: s.ask },
      ];
    case "editor":
      return [
        { id: "explainCode", label: s.explainCode },
        { id: "checkIssues", label: s.checkIssues },
        { id: "ask", label: s.ask },
      ];
    case "terminal":
      return [
        { id: "diagnoseError", label: s.diagnoseError },
        { id: "ask", label: s.ask },
      ];
  }
}

function computeNonOverlappingPosition(
  rect: ActiveSelection["rect"],
  estimatedWidth: number,
  vw: number,
  vh: number
): { top: number; left: number; placement: "above" | "below" | "side" } {
  const clampX = (x: number) =>
    Math.max(VIEWPORT_MARGIN, Math.min(vw - estimatedWidth - VIEWPORT_MARGIN, Math.round(x)));
  const centerX = (rect.left + rect.right) / 2 - estimatedWidth / 2;

  // 1. Prefer strictly above selection without clipping topbar.
  const aboveTop = rect.top - PILL_HEIGHT - PILL_GAP;
  if (aboveTop >= TOPBAR_SAFE_TOP) {
    return { top: Math.round(aboveTop), left: clampX(centerX), placement: "above" };
  }

  // 2. Otherwise place strictly below selection if viewport has room.
  const belowTop = rect.bottom + PILL_GAP;
  if (belowTop + PILL_HEIGHT <= vh - VIEWPORT_MARGIN) {
    return { top: Math.round(belowTop), left: clampX(centerX), placement: "below" };
  }

  // 3. Tall selection covering both top and bottom: place beside selection so it never covers text.
  const midY = Math.max(
    TOPBAR_SAFE_TOP,
    Math.min(vh - PILL_HEIGHT - VIEWPORT_MARGIN, Math.round((rect.top + rect.bottom) / 2 - PILL_HEIGHT / 2))
  );
  if (rect.right + PILL_GAP + estimatedWidth <= vw - VIEWPORT_MARGIN) {
    return { top: midY, left: Math.round(rect.right + PILL_GAP), placement: "side" };
  }
  if (rect.left - PILL_GAP - estimatedWidth >= VIEWPORT_MARGIN) {
    return {
      top: midY,
      left: Math.round(rect.left - PILL_GAP - estimatedWidth),
      placement: "side",
    };
  }

  // Fallback for full-viewport selections: dock cleanly against the topbar rail.
  return { top: TOPBAR_SAFE_TOP, left: clampX(centerX), placement: "above" };
}

export interface SelectionAskPillProps {
  selection: ActiveSelection | null;
  messages: Messages;
  onAction: (selection: ActiveSelection, action: SelectionQuickAction) => void;
  onDismiss: () => void;
}

export function SelectionAskPill({
  selection,
  messages,
  onAction,
  onDismiss,
}: SelectionAskPillProps) {
  const pillRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!selection) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onDismiss();
      }
    };

    const handleScroll = () => {
      onDismiss();
    };

    const handleMouseDown = (event: MouseEvent) => {
      const target = event.target as Node | null;
      if (target && pillRef.current?.contains(target)) {
        return;
      }
      onDismiss();
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("mousedown", handleMouseDown, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("mousedown", handleMouseDown, true);
    };
  }, [onDismiss, selection]);

  if (!selection) return null;

  const actions = actionsForZone(selection.zone, messages);
  const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
  const vh = typeof window !== "undefined" ? window.innerHeight : 800;
  const estimatedWidth = actions.length === 3 ? 256 : 196;
  const { top, left, placement } = computeNonOverlappingPosition(
    selection.rect,
    estimatedWidth,
    vw,
    vh
  );

  return (
    <div
      ref={pillRef}
      className="selection-ask-pill"
      role="toolbar"
      aria-label={messages.tutor.selection.ariaLabel}
      data-testid="selection-ask-pill"
      data-zone={selection.zone}
      data-placement={placement}
      style={{ top: `${top}px`, left: `${left}px` }}
    >
      <span className="selection-ask-pill-icon" aria-hidden="true">
        <SparklesIcon size={12} />
      </span>
      {actions.map((item, idx) => (
        <Fragment key={item.id}>
          {idx > 0 && <span className="selection-ask-pill-sep" aria-hidden="true" />}
          <button
            type="button"
            className="selection-ask-pill-btn"
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onAction(selection, item.id)}
          >
            {item.label}
          </button>
        </Fragment>
      ))}
    </div>
  );
}
