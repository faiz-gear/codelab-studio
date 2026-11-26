import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import type { CoreClient } from "../core/client";
import type { GlobalSettings, UpdateSettingsPatch } from "../core/contracts";
import { TutorSettingsPanel } from "../tutor/TutorSettingsPanel";
import { CloseIcon } from "./Icons";
import { RuntimePanel, type RuntimePanelProps } from "./RuntimePanel";

export type SettingsTab = "runtime" | "tutor";

export interface RuntimeSettingsDialogProps extends RuntimePanelProps {
  client: CoreClient;
  settings: GlobalSettings;
  initialTab?: SettingsTab;
  onUpdateSettings: (patch: UpdateSettingsPatch) => Promise<void> | void;
  onSavedTutorConfig?: () => void;
  onClose: () => void;
}

export function RuntimeSettingsDialog({
  client,
  settings,
  initialTab = "runtime",
  onUpdateSettings,
  onSavedTutorConfig,
  onClose,
  ...panel
}: RuntimeSettingsDialogProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const runtimeTabRef = useRef<HTMLButtonElement>(null);
  const tutorTabRef = useRef<HTMLButtonElement>(null);
  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);
  const id = useId();
  const { messages } = panel;

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    element.showModal();
    (initialTab === "tutor" ? tutorTabRef.current : runtimeTabRef.current)?.focus();
    return () => element.close();
  }, [initialTab]);

  const handleTabKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const next: SettingsTab =
      event.key === "Home"
        ? "runtime"
        : event.key === "End"
          ? "tutor"
          : activeTab === "runtime"
            ? "tutor"
            : "runtime";
    setActiveTab(next);
    (next === "tutor" ? tutorTabRef.current : runtimeTabRef.current)?.focus();
  };

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
        onKeyDown={handleTabKeyDown}
      >
        <button
          ref={runtimeTabRef}
          id={`${id}-tab-runtime`}
          type="button"
          className="btn btn--secondary btn--sm"
          role="tab"
          aria-selected={activeTab === "runtime"}
          tabIndex={activeTab === "runtime" ? 0 : -1}
          aria-controls={`${id}-panel-runtime`}
          onClick={() => setActiveTab("runtime")}
        >
          {messages.runtime.settingsTab}
        </button>
        <button
          ref={tutorTabRef}
          id={`${id}-tab-tutor`}
          type="button"
          className="btn btn--secondary btn--sm"
          role="tab"
          aria-selected={activeTab === "tutor"}
          tabIndex={activeTab === "tutor" ? 0 : -1}
          aria-controls={`${id}-panel-tutor`}
          onClick={() => setActiveTab("tutor")}
        >
          {messages.tutor.settingsTab}
        </button>
      </div>
      <div
        className="modal-body"
        id={`${id}-panel-runtime`}
        role="tabpanel"
        aria-labelledby={`${id}-tab-runtime`}
        hidden={activeTab !== "runtime"}
      >
        {activeTab === "runtime" && <RuntimePanel {...panel} expanded />}
      </div>
      <div
        className="modal-body"
        id={`${id}-panel-tutor`}
        role="tabpanel"
        aria-labelledby={`${id}-tab-tutor`}
        hidden={activeTab !== "tutor"}
      >
        {activeTab === "tutor" && (
          <TutorSettingsPanel
            client={client}
            settings={settings}
            messages={messages}
            onUpdateSettings={onUpdateSettings}
            onSavedToast={onSavedTutorConfig}
          />
        )}
      </div>
    </dialog>
  );
}
