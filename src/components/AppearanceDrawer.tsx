import { useEffect, useRef } from "react";
import {
  ACCENT_PRESETS,
  CODE_FONT_SIZE_PRESETS,
  type AccentPreset,
  type GlobalSettings,
  type Locale,
  type ThemeMode,
  type UpdateSettingsPatch,
} from "../core/contracts";
import type { Messages } from "../i18n";
import { CheckIcon, CloseIcon, CopyIcon } from "./Icons";

const ACCENT_SWATCH_COLOR: Record<AccentPreset, string> = {
  neutral: "#525252",
  violet: "#6836f5",
  blue: "#0562ef",
  green: "#077e53",
  amber: "#9c5b00",
  orange: "#b84900",
  coral: "#c5342b",
  rose: "#bc2470",
};

interface SegmentedOption<T extends string | number> {
  value: T;
  label: string;
}

interface SegmentedGroupProps<T extends string | number> {
  label: string;
  groupLabel: string;
  options: readonly SegmentedOption<T>[];
  isSelected: (value: T) => boolean;
  onSelect: (value: T) => void;
  buttonHeight?: string;
}

function SegmentedGroup<T extends string | number>({
  label,
  groupLabel,
  options,
  isSelected,
  onSelect,
  buttonHeight = "30px",
}: SegmentedGroupProps<T>) {
  return (
    <div>
      <span className="drawer-label">{label}</span>
      <div
        className="seg"
        style={{ display: "flex", width: "100%" }}
        role="group"
        aria-label={groupLabel}
      >
        {options.map((option) => (
          <button
            key={String(option.value)}
            type="button"
            aria-pressed={isSelected(option.value)}
            style={{ flex: 1, height: buttonHeight }}
            onClick={() => onSelect(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export interface AppearanceDrawerProps {
  open: boolean;
  settings: GlobalSettings;
  messages: Messages;
  onClose: () => void;
  onUpdateSettings: (patch: UpdateSettingsPatch) => void;
  onCopyThemeConfig: () => void;
}

export function AppearanceDrawer({
  open,
  settings,
  messages,
  onClose,
  onUpdateSettings,
  onCopyThemeConfig,
}: AppearanceDrawerProps) {
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!open) return;
    closeButtonRef.current?.focus();

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

  const sizeOptions: readonly SegmentedOption<number>[] = [
    { value: CODE_FONT_SIZE_PRESETS[0], label: messages.appearance.sizeCompact },
    { value: CODE_FONT_SIZE_PRESETS[1], label: messages.appearance.sizeStandard },
    { value: CODE_FONT_SIZE_PRESETS[2], label: messages.appearance.sizeLarge },
  ];

  const themeOptions: readonly SegmentedOption<ThemeMode>[] = [
    { value: "light", label: messages.appearance.themeLight },
    { value: "dark", label: messages.appearance.themeDark },
  ];

  const localeOptions: readonly SegmentedOption<Locale>[] = [
    { value: "zh-CN", label: "简体中文" },
    { value: "en-US", label: "English" },
  ];

  return (
    <div
      className="drawer-scrim"
      role="dialog"
      aria-modal="true"
      aria-label={messages.appearance.dialogLabel}
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div className="appearance-drawer">
        <div className="drawer-head">
          <h2>{messages.appearance.title}</h2>
          <button
            ref={closeButtonRef}
            type="button"
            className="btn btn--secondary btn--icon btn--xs btn--pill"
            aria-label={messages.appearance.close}
            onClick={onClose}
          >
            <CloseIcon />
          </button>
        </div>

        <div className="drawer-body">
          <SegmentedGroup
            label={messages.appearance.themeLabel}
            groupLabel={messages.appearance.themeGroupLabel}
            options={themeOptions}
            isSelected={(value) => settings.theme === value}
            onSelect={(theme) => onUpdateSettings({ theme })}
          />

          <div>
            <span className="drawer-label">{messages.appearance.accentLabel}</span>
            <div
              className="swatch-row"
              role="group"
              aria-label={messages.appearance.accentGroupLabel}
            >
              {ACCENT_PRESETS.map((preset) => {
                const selected = settings.accent === preset;
                return (
                  <button
                    key={preset}
                    type="button"
                    className="swatch"
                    aria-label={messages.appearance.accents[preset]}
                    aria-pressed={selected}
                    title={messages.appearance.accents[preset]}
                    style={{ background: ACCENT_SWATCH_COLOR[preset] }}
                    onClick={() => onUpdateSettings({ accent: preset })}
                  >
                    {selected ? <CheckIcon /> : null}
                  </button>
                );
              })}
            </div>
            <button
              type="button"
              className="btn btn--secondary btn--sm btn--pill"
              style={{ marginTop: "14px" }}
              onClick={onCopyThemeConfig}
            >
              <CopyIcon />
              <span>{messages.appearance.copyConfig}</span>
            </button>
          </div>

          <div>
            <SegmentedGroup
              label={messages.appearance.codeSizeLabel}
              groupLabel={messages.appearance.codeSizeGroupLabel}
              options={sizeOptions}
              buttonHeight="28px"
              isSelected={(value) => Math.abs(settings.codeFontSize - value) < 0.01}
              onSelect={(codeFontSize) => onUpdateSettings({ codeFontSize })}
            />
            <pre
              className="code-preview"
              aria-label={messages.appearance.codePreviewLabel}
            >
              <code>{`func main() {\n  http.ListenAndServe(":8080", mux)\n}`}</code>
            </pre>
          </div>

          <SegmentedGroup
            label={messages.appearance.localeLabel}
            groupLabel={messages.appearance.localeGroupLabel}
            options={localeOptions}
            isSelected={(value) => settings.locale === value}
            onSelect={(locale) => onUpdateSettings({ locale })}
          />
        </div>
      </div>
    </div>
  );
}
