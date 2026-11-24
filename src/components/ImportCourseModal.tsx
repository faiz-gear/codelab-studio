import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { LocalPathPickKind, OpenCourseRequest } from "../core/contracts";
import type { Messages } from "../i18n";
import { CloseIcon, FolderDownIcon, FolderIcon } from "./Icons";

export type ImportMode = "localMarkdown" | "localPack";

export interface ImportCourseModalProps {
  open: boolean;
  messages: Messages;
  errorText: string | null;
  submitting: boolean;
  onClose: () => void;
  onBrowsePath: (kind: LocalPathPickKind) => Promise<string | null>;
  onSubmit: (request: OpenCourseRequest) => Promise<void>;
  onClearError: () => void;
}

export function ImportCourseModal({
  open,
  messages,
  errorText,
  submitting,
  onClose,
  onBrowsePath,
  onSubmit,
  onClearError,
}: ImportCourseModalProps) {
  const [mode, setMode] = useState<ImportMode>("localMarkdown");
  const [pathValue, setPathValue] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const inputId = useId();
  const titleId = useId();

  const handleClose = useCallback(() => {
    setLocalError(null);
    onClose();
  }, [onClose]);

  useEffect(() => {
    if (!open) return;
    const frame = window.requestAnimationFrame(() => {
      inputRef.current?.focus();
    });
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        handleClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, handleClose]);

  if (!open) return null;

  const t = messages.importModal;
  const activeError = localError ?? errorText;

  const handleModeChange = (nextMode: ImportMode) => {
    setMode(nextMode);
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

  const handleFormSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    const trimmed = pathValue.trim();
    if (!trimmed) {
      setLocalError(t.pathRequired);
      return;
    }
    setLocalError(null);
    const request: OpenCourseRequest =
      mode === "localMarkdown"
        ? { kind: "localMarkdown", path: trimmed }
        : { kind: "localPack", path: trimmed };
    await onSubmit(request);
  };

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
        aria-label={t.dialogLabel}
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
              <button
                type="button"
                aria-pressed={mode === "localMarkdown"}
                onClick={() => handleModeChange("localMarkdown")}
              >
                {t.modeMarkdown}
              </button>
              <button
                type="button"
                aria-pressed={mode === "localPack"}
                onClick={() => handleModeChange("localPack")}
              >
                {t.modePack}
              </button>
            </div>

            <div className="field">
              <label htmlFor={inputId}>
                {mode === "localMarkdown"
                  ? t.pathLabelMarkdown
                  : t.pathLabelPack}
              </label>
              <div className="field-row">
                <input
                  ref={inputRef}
                  id={inputId}
                  type="text"
                  className="text-input mono"
                  value={pathValue}
                  placeholder={
                    mode === "localMarkdown"
                      ? t.pathPlaceholderMarkdown
                      : t.pathPlaceholderPack
                  }
                  onChange={(event) => {
                    setPathValue(event.target.value);
                    if (localError) setLocalError(null);
                    if (errorText) onClearError();
                  }}
                />
                <button
                  type="button"
                  className="btn btn--secondary btn--md"
                  onClick={() => void handleBrowse()}
                >
                  {mode === "localMarkdown" ? (
                    <FolderDownIcon />
                  ) : (
                    <FolderIcon />
                  )}
                  <span>
                    {mode === "localMarkdown" ? t.browseFile : t.browseDir}
                  </span>
                </button>
              </div>
              <p className="field-hint">
                {mode === "localMarkdown" ? t.hintMarkdown : t.hintPack}
              </p>
            </div>

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
            >
              {t.cancel}
            </button>
            <button
              type="submit"
              className="btn btn--primary btn--pill"
              disabled={submitting}
            >
              {submitting ? t.submitting : t.submit}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
