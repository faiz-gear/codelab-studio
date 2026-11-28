import { useId } from "react";
import {
  ACCENT_PRESETS,
  CODE_FONT_SIZE_PRESETS,
  EXTERNAL_IDES,
  type ExternalIde,
  type GlobalSettings,
  type Locale,
  type ThemeMode,
  type UpdateSettingsPatch,
} from "../core/contracts";
import type { Messages } from "../i18n";
import { ACCENT_SWATCH_COLOR } from "../theme/applyTheme";
import { CheckIcon, CopyIcon } from "./Icons";
import { Select } from "./Select";

export interface EditorSettingsPanelProps {
  settings: GlobalSettings;
  messages: Messages;
  onUpdateSettings: (patch: UpdateSettingsPatch) => Promise<void> | void;
  onCopyThemeConfig: () => void;
}

export function EditorSettingsPanel({
  settings,
  messages,
  onUpdateSettings,
  onCopyThemeConfig,
}: EditorSettingsPanelProps) {
  const id = useId();
  const a = messages.appearance;
  const e = messages.settings.editor;

  const themeOptions: readonly { value: ThemeMode; label: string }[] = [
    { value: "light", label: a.themeLight },
    { value: "dark", label: a.themeDark },
  ];

  const sizeOptions: readonly { value: number; label: string }[] = [
    { value: CODE_FONT_SIZE_PRESETS[0], label: a.sizeCompact },
    { value: CODE_FONT_SIZE_PRESETS[1], label: a.sizeStandard },
    { value: CODE_FONT_SIZE_PRESETS[2], label: a.sizeLarge },
  ];

  const localeOptions: readonly { value: Locale; label: string }[] = [
    { value: "zh-CN", label: "简体中文" },
    { value: "en-US", label: "English" },
  ];

  return (
    <div className="settings-section-stack" data-testid="editor-settings-panel">
      <p className="settings-intro">{e.intro}</p>

      <div className="settings-two-col">
        <div className="field">
          <span className="tutor-field-label">{a.themeLabel}</span>
          <div className="seg settings-seg" role="group" aria-label={a.themeGroupLabel}>
            {themeOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                aria-pressed={settings.theme === option.value}
                onClick={() => void onUpdateSettings({ theme: option.value })}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        <div className="field">
          <span className="tutor-field-label">{a.localeLabel}</span>
          <div className="seg settings-seg" role="group" aria-label={a.localeGroupLabel}>
            {localeOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                aria-pressed={settings.locale === option.value}
                onClick={() => void onUpdateSettings({ locale: option.value })}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="field">
        <span className="tutor-field-label">{a.accentLabel}</span>
        <div className="settings-accent-row">
          <div className="swatch-row" role="group" aria-label={a.accentGroupLabel}>
            {ACCENT_PRESETS.map((preset) => {
              const selected = settings.accent === preset;
              return (
                <button
                  key={preset}
                  type="button"
                  className="swatch"
                  aria-label={a.accents[preset]}
                  aria-pressed={selected}
                  title={a.accents[preset]}
                  style={{ background: ACCENT_SWATCH_COLOR[preset] }}
                  onClick={() => void onUpdateSettings({ accent: preset })}
                >
                  {selected ? <CheckIcon /> : null}
                </button>
              );
            })}
          </div>
          <button
            type="button"
            className="btn btn--secondary btn--sm btn--pill"
            onClick={onCopyThemeConfig}
          >
            <CopyIcon />
            <span>{a.copyConfig}</span>
          </button>
        </div>
      </div>

      <div className="field">
        <span className="tutor-field-label">{a.codeSizeLabel}</span>
        <div className="seg settings-seg" role="group" aria-label={a.codeSizeGroupLabel}>
          {sizeOptions.map((option) => (
            <button
              key={option.value}
              type="button"
              aria-pressed={Math.abs(settings.codeFontSize - option.value) < 0.01}
              onClick={() => void onUpdateSettings({ codeFontSize: option.value })}
            >
              {option.label}
            </button>
          ))}
        </div>
        <pre className="code-preview" aria-label={a.codePreviewLabel}>
          <code>{`func main() {\n  http.ListenAndServe(":8080", mux)\n}`}</code>
        </pre>
      </div>

      <div className="field">
        <label htmlFor={`${id}-ide`}>{e.externalIdeLabel}</label>
        <Select<ExternalIde>
          id={`${id}-ide`}
          value={settings.preferredIde}
          options={EXTERNAL_IDES.map((ide) => ({ value: ide.id, label: ide.label }))}
          onChange={(preferredIde) => void onUpdateSettings({ preferredIde })}
        />
        <p className="field-hint">{e.externalIdeHint}</p>
      </div>

      <section className="settings-card" aria-labelledby={`${id}-shortcuts`}>
        <h3 id={`${id}-shortcuts`}>{e.shortcutsHeading}</h3>
        <dl className="settings-shortcuts-list">
          <div className="settings-shortcut-row">
            <dt>{e.shortcutFocusMode}</dt>
            <dd>
              <kbd className="mono">⌘/Ctrl + B</kbd>
            </dd>
          </div>
          <div className="settings-shortcut-row">
            <dt>{e.shortcutSaveFile}</dt>
            <dd>
              <kbd className="mono">⌘/Ctrl + S</kbd>
            </dd>
          </div>
          <div className="settings-shortcut-row">
            <dt>{e.shortcutCloseOverlay}</dt>
            <dd>
              <kbd className="mono">Esc</kbd>
            </dd>
          </div>
        </dl>
      </section>
    </div>
  );
}
