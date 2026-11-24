import { useEffect } from "react";
import type { Chapter } from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { CloseIcon } from "./Icons";

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
  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const t = messages.workbench;

  return (
    <div
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
          type="button"
          className="btn btn--ghost btn--icon btn--xs"
          aria-label={t.outlineClose}
          onClick={onClose}
        >
          <CloseIcon size={13} />
        </button>
      </div>

      <ul className="outline-list">
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
