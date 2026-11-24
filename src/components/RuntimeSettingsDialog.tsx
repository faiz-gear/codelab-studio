import { useEffect, useId, useRef } from "react";
import { CloseIcon } from "./Icons";
import { RuntimePanel, type RuntimePanelProps } from "./RuntimePanel";

export function RuntimeSettingsDialog({
  onClose,
  ...panel
}: RuntimePanelProps & { onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const tab = useRef<HTMLButtonElement>(null);
  const id = useId();
  const { messages } = panel;

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    element.showModal();
    tab.current?.focus();
    return () => element.close();
  }, []);

  return (
    <dialog
      ref={dialog}
      className="modal-card runtime-settings"
      aria-modal="true"
      aria-labelledby={`${id}-title`}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <header className="modal-head">
        <h2 id={`${id}-title`}>{messages.topbar.settingsLabel}</h2>
        <button
          type="button"
          className="btn btn--ghost btn--icon btn--xs"
          aria-label={messages.runtime.close}
          onClick={onClose}
        >
          <CloseIcon size={14} />
        </button>
      </header>
      <div
        className="runtime-settings-tabs"
        role="tablist"
        aria-label={messages.topbar.settingsLabel}
      >
        <button
          ref={tab}
          id={`${id}-tab`}
          type="button"
          className="btn btn--secondary btn--sm"
          role="tab"
          aria-selected="true"
          aria-controls={`${id}-panel`}
        >
          {messages.runtime.settingsTab}
        </button>
      </div>
      <div
        className="modal-body"
        id={`${id}-panel`}
        role="tabpanel"
        aria-labelledby={`${id}-tab`}
      >
        <RuntimePanel {...panel} expanded />
      </div>
    </dialog>
  );
}
