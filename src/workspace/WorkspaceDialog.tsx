import { type ReactNode, useEffect, useId, useRef } from "react";

export function WorkspaceDialog({
  title, detail, cancel, children, onCancel, body, className, busy = false,
}: {
  title: string;
  detail?: ReactNode;
  cancel: string;
  children: ReactNode;
  body?: ReactNode;
  className?: string;
  onCancel: () => void;
  busy?: boolean;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const cancelButton = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const detailId = useId();
  useEffect(() => {
    const el = dialog.current;
    const previous = document.activeElement;
    if (el?.showModal) el.showModal();
    else el?.setAttribute("open", "");
    cancelButton.current?.focus();
    return () => {
      if (el?.close) el.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`modal-card workspace-dialog${className ? ` ${className}` : ""}`}
      aria-labelledby={titleId}
      aria-describedby={detail ? detailId : undefined}
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onCancel();
      }}
    >
      <div className="modal-head">
        <div className="modal-title-group">
          <h2 id={titleId}>{title}</h2>
          {detail && <div className="modal-sub" id={detailId}>{detail}</div>}
        </div>
      </div>
      {body}
      <div className="modal-foot">
        <button ref={cancelButton} className="btn btn--secondary btn--sm" type="button" disabled={busy} onClick={onCancel}>{cancel}</button>
        {children}
      </div>
    </dialog>
  );
}
