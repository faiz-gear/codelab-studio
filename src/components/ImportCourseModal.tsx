import { useCallback, useEffect, useId, useRef, useState } from "react";
import {
  COURSE_LANGUAGES, type AppCommand, type CourseLanguage, type LocalPathPickKind, type OpenCourseRequest,
} from "../core/contracts";
import type { Messages } from "../i18n";
import { CloseIcon, FolderDownIcon, FolderIcon } from "./Icons";

export type ImportMode = "localMarkdown" | "localPack" | "gitPack" | "author";
export type DraftOpenCommand = Extract<AppCommand, { type: "createCourseDraft" | "openCourseDraft" }>;

export interface ImportCourseModalProps {
  initialMode: ImportMode;
  messages: Messages;
  errorText: string | null;
  submitting: boolean;
  onClose: () => void;
  onBrowsePath: (kind: LocalPathPickKind) => Promise<string | null>;
  onSubmit: (request: OpenCourseRequest) => Promise<void>;
  onOpenDraft: (command: DraftOpenCommand) => Promise<void>;
  onClearError: () => void;
}

export function ImportCourseModal({
  initialMode,
  messages,
  errorText,
  submitting,
  onClose,
  onBrowsePath,
  onSubmit,
  onOpenDraft,
  onClearError,
}: ImportCourseModalProps) {
  const [mode, setMode] = useState(initialMode);
  const [title, setTitle] = useState("");
  const [language, setLanguage] = useState<CourseLanguage>("Go");
  const [referenceSolution, setReferenceSolution] = useState(false);
  const [pathValue, setPathValue] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const inputId = useId();
  const titleId = useId();

  const handleClose = useCallback(() => {
    if (!submitting) onClose();
  }, [onClose, submitting]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => inputRef.current?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        handleClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleClose]);

  const t = messages.importModal;
  const p = messages.coursePack;
  const git = mode === "gitPack";
  const author = mode === "author";
  const markdown = mode === "localMarkdown";
  const activeError = localError ?? errorText;
  const labels = {
    localMarkdown: t.modeMarkdown,
    localPack: t.modePack,
    gitPack: p.gitMode,
    author: p.authorMode,
  };

  const handleModeChange = (nextMode: ImportMode) => {
    setMode(nextMode);
    setPathValue("");
    setLocalError(null);
    onClearError();
  };

  const handleBrowse = async () => {
    setLocalError(null);
    onClearError();
    const pickKind: LocalPathPickKind =
      mode === "localMarkdown" ? "markdownFile" : "coursePackDir";
    const picked = await onBrowsePath(pickKind);
    if (picked) {
      setPathValue(picked);
    }
  };

  function currentPath() {
    const path = pathValue.trim();
    setLocalError(path ? null : git ? p.gitUrlRequired : t.pathRequired);
    return path;
  }

  async function handleFormSubmit(event: React.FormEvent) {
    event.preventDefault();
    if (submitting) return;
    const path = currentPath();
    if (!path) return;
    if (author) {
      await onOpenDraft({ type: "createCourseDraft", payload: { path, title, language, referenceSolution } });
    } else {
      await onSubmit(git ? { kind: mode, url: path } : { kind: mode, path });
    }
  }

  async function openExistingDraft() {
    const path = currentPath();
    if (path) await onOpenDraft({ type: "openCourseDraft", payload: { path } });
  }

  return (
    <div
      className="modal-scrim"
      role="presentation"
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          handleClose();
        }
      }}
    >
      <div
        className="modal-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="modal-head">
          <div className="modal-title-group">
            <h2 id={titleId}>{t.title}</h2>
            <p className="modal-sub">{t.subtitle}</p>
          </div>
          <button
            type="button"
            className="btn btn--ghost btn--icon btn--xs"
            aria-label={t.close}
            onClick={handleClose}
          >
            <CloseIcon />
          </button>
        </div>

        <form onSubmit={(event) => void handleFormSubmit(event)}>
          <div className="modal-body">
            <div
              className="seg seg--full"
              role="group"
              aria-label={t.modeGroupLabel}
            >
              {(Object.keys(labels) as ImportMode[]).map((option) => (
                <button key={option} type="button" disabled={submitting}
                  aria-pressed={mode === option} onClick={() => handleModeChange(option)}>
                  {labels[option]}
                </button>
              ))}
            </div>

            {author && (
              <>
                <label className="field">
                  <span>{p.courseName}</span>
                  <input className="text-input" value={title} required disabled={submitting}
                    onChange={(event) => setTitle(event.target.value)} />
                </label>
                <label className="field">
                  <span>{p.language}</span>
                  <select className="text-input" value={language} disabled={submitting}
                    onChange={(event) => setLanguage(event.target.value as CourseLanguage)}>
                    {COURSE_LANGUAGES.map((value) => <option key={value}>{value}</option>)}
                  </select>
                </label>
              </>
            )}

            <div className="field">
              <label htmlFor={inputId}>
                {git ? p.gitUrl : author ? p.draftDirectory : markdown ? t.pathLabelMarkdown : t.pathLabelPack}
              </label>
              <div className="field-row">
                <input ref={inputRef} id={inputId} type="text" className="text-input mono"
                  value={pathValue} disabled={submitting}
                  placeholder={git ? "https://example.org/team/course.git" : markdown ? t.pathPlaceholderMarkdown : t.pathPlaceholderPack}
                  onChange={(event) => {
                    setPathValue(event.target.value);
                    if (localError) setLocalError(null);
                    if (errorText) onClearError();
                  }}
                />
                {!git && (
                  <button type="button" className="btn btn--secondary btn--md" disabled={submitting}
                    onClick={() => void handleBrowse()}>
                    {markdown ? <FolderDownIcon /> : <FolderIcon />}
                    <span>{markdown ? t.browseFile : t.browseDir}</span>
                  </button>
                )}
              </div>
              <p className="field-hint">{git ? p.gitHint : author ? p.draftHint : markdown ? t.hintMarkdown : t.hintPack}</p>
            </div>
            {author && (
              <label className="draft-checkbox">
                <input type="checkbox" checked={referenceSolution} disabled={submitting}
                  onChange={(event) => setReferenceSolution(event.target.checked)} />
                {p.referenceSolution}
              </label>
            )}

            {activeError ? (
              <div className="inline-alert inline-alert--danger" role="alert">
                {activeError}
              </div>
            ) : null}
          </div>

          <div className="modal-foot">
            <button
              type="button"
              className="btn btn--secondary btn--pill"
              onClick={handleClose}
              disabled={submitting}
            >
              {t.cancel}
            </button>
            {author && <button type="button" className="btn btn--secondary btn--pill"
              disabled={submitting} onClick={() => void openExistingDraft()}>{p.openDraft}</button>}
            <button
              type="submit"
              className="btn btn--primary btn--pill"
              disabled={submitting}
            >
              {submitting ? (git ? p.gitLoading : t.submitting) : author ? p.createDraft : t.submit}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
