import type { RefObject } from "react";
import type { Messages } from "../i18n";
import { BrandMarkIcon, SettingsIcon, SlidersIcon } from "./Icons";

export interface TopBarProps {
  messages: Messages;
  appearanceOpen: boolean;
  appearanceTriggerRef: RefObject<HTMLButtonElement | null>;
  onOpenAppearance: () => void;
  onPlaceholderAction: (label: string) => void;
}

export function TopBar({
  messages,
  appearanceOpen,
  appearanceTriggerRef,
  onOpenAppearance,
  onPlaceholderAction,
}: TopBarProps) {
  return (
    <header className="topbar">
      <button
        type="button"
        className="brand"
        title={messages.topbar.homeTooltip}
        onClick={() => window.scrollTo({ top: 0 })}
      >
        <span className="brand-mark">
          <BrandMarkIcon />
        </span>
        <span>{messages.brandTitle}</span>
      </button>

      <div className="top-right">
        <button
          type="button"
          className="btn btn--ghost btn--sm btn--pill"
          title={messages.topbar.envStatusTooltip}
          onClick={() => onPlaceholderAction(messages.topbar.envStatusLabel)}
        >
          <span className="dot dot--muted" aria-hidden="true" />
          <span>{messages.topbar.envStatusLabel}</span>
        </button>

        <button
          ref={appearanceTriggerRef}
          type="button"
          className="btn btn--ghost btn--sm btn--pill"
          title={messages.topbar.appearanceTooltip}
          aria-expanded={appearanceOpen}
          aria-haspopup="dialog"
          onClick={onOpenAppearance}
        >
          <SlidersIcon />
          <span>{messages.topbar.appearance}</span>
        </button>

        <button
          type="button"
          className="btn btn--ghost btn--icon btn--sm btn--pill"
          aria-label={messages.topbar.settingsLabel}
          title={messages.topbar.settingsTooltip}
          onClick={() => onPlaceholderAction(messages.topbar.settingsLabel)}
        >
          <SettingsIcon />
        </button>
      </div>
    </header>
  );
}
