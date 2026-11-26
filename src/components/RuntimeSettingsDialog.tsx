import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import type { CoreClient } from "../core/client";
import type {
  GlobalSettings,
  TrustEntry,
  TrustSource,
  UpdateSettingsPatch,
  WorkspaceSummary,
} from "../core/contracts";
import { TutorSettingsPanel } from "../tutor/TutorSettingsPanel";
import { EditorSettingsPanel } from "./EditorSettingsPanel";
import { CloseIcon } from "./Icons";
import { PrivacySettingsPanel } from "./PrivacySettingsPanel";
import { RuntimePanel, type RuntimePanelProps } from "./RuntimePanel";
import { trapFocusOnTab } from "./focusTrap";

export type SettingsTab = "tutor" | "runtime" | "editor" | "privacy";

const SETTINGS_TABS: readonly SettingsTab[] = [
  "tutor",
  "runtime",
  "editor",
  "privacy",
] as const;

export interface RuntimeSettingsDialogProps extends RuntimePanelProps {
  client: CoreClient;
  settings: GlobalSettings;
  initialTab?: SettingsTab;
  trustEntries?: TrustEntry[];
  activeWorkspace?: WorkspaceSummary | null;
  activeCourseTrusted?: boolean;
  onUpdateSettings: (patch: UpdateSettingsPatch) => Promise<void> | void;
  onCopyThemeConfig?: () => void;
  onSavedTutorConfig?: () => void;
  onRevokeTrustEntry?: (source: TrustSource) => Promise<void>;
  onClearTutorHistory?: (workspaceId: number) => Promise<void>;
  onResetWorkspaceProgress?: (workspaceId: number) => Promise<void>;
  onClose: () => void;
}

const noopAsync = async () => {};
const noop = () => {};

export function RuntimeSettingsDialog({
  client,
  settings,
  initialTab = "runtime",
  trustEntries = [],
  activeWorkspace = null,
  activeCourseTrusted = false,
  onUpdateSettings,
  onCopyThemeConfig = noop,
  onSavedTutorConfig,
  onRevokeTrustEntry = noopAsync,
  onClearTutorHistory = noopAsync,
  onResetWorkspaceProgress = noopAsync,
  onClose,
  ...panel
}: RuntimeSettingsDialogProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const tabRefs = useRef<Record<SettingsTab, HTMLButtonElement | null>>({
    tutor: null,
    runtime: null,
    editor: null,
    privacy: null,
  });
  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);
  const id = useId();
  const { messages } = panel;

  useEffect(() => {
    tabRefs.current[initialTab]?.focus();
  }, [initialTab]);

  useEffect(() => {
    const handleKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      } else if (event.key === "Tab") {
        trapFocusOnTab(dialog.current, event);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const handleTabKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (
      !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(
        event.key
      )
    ) {
      return;
    }
    event.preventDefault();
    const index = SETTINGS_TABS.indexOf(activeTab);
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? SETTINGS_TABS.length - 1
          : event.key === "ArrowLeft" || event.key === "ArrowUp"
            ? (index - 1 + SETTINGS_TABS.length) % SETTINGS_TABS.length
            : (index + 1) % SETTINGS_TABS.length;
    const next = SETTINGS_TABS[nextIndex];
    setActiveTab(next);
    tabRefs.current[next]?.focus();
  };

  return (
    <div
      className="modal-scrim"
      role="presentation"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <dialog
        ref={dialog}
        open
        className="modal-card runtime-settings"
        aria-modal="true"
        aria-labelledby={`${id}-title`}
        onCancel={(event) => {
          event.preventDefault();
          onClose();
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
      <div className="settings-layout">
        <div
          className="runtime-settings-tabs"
          role="tablist"
          aria-label={messages.topbar.settingsLabel}
          onKeyDown={handleTabKeyDown}
        >
          {SETTINGS_TABS.map((tab) => {
            const selected = activeTab === tab;
            return (
              <button
                key={tab}
                ref={(el) => {
                  tabRefs.current[tab] = el;
                }}
                id={`${id}-tab-${tab}`}
                type="button"
                className="settings-nav-btn"
                role="tab"
                aria-selected={selected}
                tabIndex={selected ? 0 : -1}
                aria-controls={`${id}-panel-${tab}`}
                onClick={() => setActiveTab(tab)}
              >
                {messages.settings.tabs[tab]}
              </button>
            );
          })}
        </div>

        <div className="settings-panel-host">
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
            id={`${id}-panel-editor`}
            role="tabpanel"
            aria-labelledby={`${id}-tab-editor`}
            hidden={activeTab !== "editor"}
          >
            {activeTab === "editor" && (
              <EditorSettingsPanel
                settings={settings}
                messages={messages}
                onUpdateSettings={onUpdateSettings}
                onCopyThemeConfig={onCopyThemeConfig}
              />
            )}
          </div>

          <div
            className="modal-body"
            id={`${id}-panel-privacy`}
            role="tabpanel"
            aria-labelledby={`${id}-tab-privacy`}
            hidden={activeTab !== "privacy"}
          >
            {activeTab === "privacy" && (
              <PrivacySettingsPanel
                client={client}
                locale={settings.locale}
                messages={messages}
                trustEntries={trustEntries}
                activeWorkspace={activeWorkspace}
                activeCourseTrusted={activeCourseTrusted}
                onRevokeTrustEntry={onRevokeTrustEntry}
                onClearTutorHistory={onClearTutorHistory}
                onResetWorkspaceProgress={onResetWorkspaceProgress}
              />
            )}
          </div>
        </div>
      </div>
      </dialog>
    </div>
  );
}
