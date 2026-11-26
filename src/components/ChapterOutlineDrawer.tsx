import { useEffect, useRef } from "react";
import type { Chapter } from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { CloseIcon } from "./Icons";
import { moveVerticalListFocus, trapFocusOnTab } from "./focusTrap";

export interface ChapterOutlineDrawerProps {
  open: boolean;
  chapters: Chapter[];
  totalExercises: number;
  activeChapterId: string;
  messages: Messages;
  onSelectChapter: (chapterId: string) => void;
  onClose: () => void;
}

export function ChapterOutlineDrawer({
  open,
  chapters,
  totalExercises,
  activeChapterId,
  messages,
  onSelectChapter,
  onClose,
}: ChapterOutlineDrawerProps) {
  const drawerRef = useRef<HTMLDivElement | null>(null);
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);

  // Focus capture, initial focus, and focus restoration run strictly on open/close transitions.
  useEffect(() => {
    if (!open) return;
    const previousFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    queueMicrotask(() => {
      const activeItem = drawerRef.current?.querySelector<HTMLButtonElement>(
        ".ch-item.is-active"
      );
      (activeItem ?? closeButtonRef.current)?.focus();
    });
    return () => {
      if (previousFocus?.isConnected) {
        queueMicrotask(() => previousFocus.focus());
      }
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      } else if (event.key === "Tab") {
        trapFocusOnTab(drawerRef.current, event);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, onClose]);

  if (!open) return null;

  const t = messages.workbench;

  return (
    <div
      ref={drawerRef}
      className="outline-drawer"
      role="dialog"
      aria-label={t.outlineTitle}
      data-testid="chapter-outline-drawer"
    >
      <div className="outline-head">
        <div>
          <strong className="outline-title">{t.outlineTitle}</strong>
          <p className="outline-summary mono">
            {interpolate(t.outlineSummary, {
              chapters: chapters.length,
              exercises: totalExercises,
            })}
          </p>
        </div>
        <button
          ref={closeButtonRef}
          type="button"
          className="btn btn--ghost btn--icon btn--xs"
          aria-label={t.outlineClose}
          onClick={onClose}
        >
          <CloseIcon size={13} />
        </button>
      </div>

      <ul
        className="outline-list"
        onKeyDown={(event) => moveVerticalListFocus(event.currentTarget, ".ch-item", event)}
      >
        {chapters.map((chapter, index) => {
          const isActive = chapter.chapterId === activeChapterId;
          const isGuide = chapter.kind === "guide";
          return (
            <li key={chapter.chapterId}>
              <button
                type="button"
                aria-current={isActive ? "step" : undefined}
                className={`ch-item${isActive ? " is-active" : ""}`}
                onClick={() => {
                  onSelectChapter(chapter.chapterId);
                  onClose();
                }}
              >
                <span className="ch-index mono">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span className="ch-name">{chapter.title}</span>
                <span
                  className={`badge ch-kind ${
                    isGuide ? "badge--neutral" : "badge--accent"
                  }`}
                >
                  {isGuide ? t.chapterKindGuide : t.chapterKindExercise}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
