import { lazy, Suspense } from "react";
import type { GlobalSettings, TutorFileChange } from "../core/contracts";
import type { Messages } from "../i18n";
import { WorkspaceDialog } from "../workspace/WorkspaceDialog";

const MonacoDiff = lazy(() =>
  import("../workspace/MonacoSurface").then((module) => ({
    default: module.MonacoDiffSurface,
  }))
);

export interface TutorFileDiffDialogProps {
  change: TutorFileChange;
  settings: GlobalSettings;
  messages: Messages;
  onClose: () => void;
  onOpenInEditor?: (path: string) => void;
}

export function TutorFileDiffDialog({
  change,
  settings,
  messages,
  onClose,
  onOpenInEditor,
}: TutorFileDiffDialogProps) {
  const t = messages.tutor;
  const w = messages.workspace;
  const before = change.beforeText ?? "";
  const useMonaco = typeof ResizeObserver !== "undefined";

  return (
    <WorkspaceDialog
      title={t.diffDialogTitle}
      detail={<span className="mono">{change.path}</span>}
      cancel={w.cancel}
      onCancel={onClose}
      className="workspace-snapshot-dialog"
      body={
        <div className="workspace-snapshot-body">
          <div
            className="workspace-diff workspace-snapshot-diff"
            role="region"
            aria-label={`${t.diffBefore} · ${t.diffAfter}`}
          >
            <div className="workspace-diff-head">
              <div>
                <strong>{t.diffBefore}</strong>
                {change.beforeText === null && (
                  <span className="workspace-snapshot-note">{t.newFileBefore}</span>
                )}
              </div>
              <strong>{t.diffAfter}</strong>
            </div>
            {useMonaco ? (
              <Suspense fallback={<p role="status">{w.loading}</p>}>
                <MonacoDiff
                  left={before}
                  right={change.afterText}
                  leftPath={change.path}
                  rightPath={change.path}
                  leftLabel={t.diffBefore}
                  rightLabel={t.diffAfter}
                  settings={settings}
                  showWhitespace
                />
              </Suspense>
            ) : (
              <div className="workspace-diff-head mono">
                <pre>{before}</pre>
                <pre>{change.afterText}</pre>
              </div>
            )}
          </div>
        </div>
      }
    >
      {onOpenInEditor && (
        <button
          type="button"
          className="btn btn--primary btn--sm"
          onClick={() => {
            onOpenInEditor(change.path);
            onClose();
          }}
        >
          {t.openInEditor}
        </button>
      )}
    </WorkspaceDialog>
  );
}
