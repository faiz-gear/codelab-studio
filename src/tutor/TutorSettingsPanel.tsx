import { useId, useState } from "react";
import { expectResponse, type CoreClient } from "../core/client";
import {
  ACP_AGENT_PRESETS,
  type AcpAgentPreset,
  type DomainError,
  type GlobalSettings,
  type PermissionPolicy,
  type TeachingStyle,
  type TutorHandshakeResult,
  type UpdateSettingsPatch,
} from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { domainError } from "../workspace/editorSession";
import { TutorErrorNotice } from "./TutorErrorNotice";

export interface TutorSettingsPanelProps {
  client: CoreClient;
  settings: GlobalSettings;
  messages: Messages;
  onUpdateSettings: (patch: UpdateSettingsPatch) => Promise<void> | void;
  onSavedToast?: () => void;
}

export function TutorSettingsPanel({
  client,
  settings,
  messages,
  onUpdateSettings,
  onSavedToast,
}: TutorSettingsPanelProps) {
  const t = messages.tutor;
  const id = useId();
  const [preset, setPreset] = useState<AcpAgentPreset>(settings.tutor.preset);
  const [command, setCommand] = useState(settings.tutor.command);
  const [syncedTutor, setSyncedTutor] = useState(settings.tutor);
  if (
    syncedTutor.preset !== settings.tutor.preset ||
    syncedTutor.command !== settings.tutor.command
  ) {
    setSyncedTutor(settings.tutor);
    setPreset(settings.tutor.preset);
    setCommand(settings.tutor.command);
  }
  const [testing, setTesting] = useState(false);
  const [handshakeResult, setHandshakeResult] = useState<TutorHandshakeResult | null>(null);
  const [handshakeError, setHandshakeError] = useState<DomainError | null>(null);

  const handlePresetChange = (nextPreset: AcpAgentPreset) => {
    setPreset(nextPreset);
    setHandshakeResult(null);
    setHandshakeError(null);
    const meta = ACP_AGENT_PRESETS.find((item) => item.id === nextPreset);
    const nextCommand = nextPreset === "custom" ? command : (meta?.defaultCommand ?? "");
    setCommand(nextCommand);
    if (nextPreset !== "custom") {
      void onUpdateSettings({ tutor: { preset: nextPreset, command: nextCommand } });
    }
  };

  const handleSaveCommand = async (event: React.FormEvent) => {
    event.preventDefault();
    setHandshakeError(null);
    await onUpdateSettings({ tutor: { preset, command: command.trim() } });
    onSavedToast?.();
  };

  const handlePolicyChange = (permissionPolicy: PermissionPolicy) => {
    void onUpdateSettings({ tutor: { permissionPolicy } });
  };

  const handleStyleChange = (teachingStyle: TeachingStyle) => {
    void onUpdateSettings({ tutor: { teachingStyle } });
  };

  const handleTestHandshake = async () => {
    setTesting(true);
    setHandshakeResult(null);
    setHandshakeError(null);
    try {
      const trimmed = command.trim();
      await onUpdateSettings({ tutor: { preset, command: trimmed } });
      const response = await client.dispatch({
        type: "testTutorHandshake",
        payload: { preset, command: trimmed },
      });
      const result = expectResponse(response, "tutorHandshake").payload;
      setHandshakeResult(result);
    } catch (error) {
      setHandshakeError(domainError(error));
    } finally {
      setTesting(false);
    }
  };

  const resolvedAgentLabel =
    handshakeResult?.agentName ??
    ACP_AGENT_PRESETS.find((item) => item.id === preset)?.label ??
    t.presets[preset];

  return (
    <div className="tutor-settings" data-testid="tutor-settings-panel">
      <div className="tutor-settings-banner">
        <span className="badge badge--ai">{t.protocolBadge}</span>
        <p className="tutor-settings-intro">{t.settingsIntro}</p>
      </div>

      <form className="tutor-settings-form" onSubmit={(event) => void handleSaveCommand(event)}>
        <div className="field">
          <label htmlFor={`${id}-preset`}>{t.presetLabel}</label>
          <select
            id={`${id}-preset`}
            className="text-input"
            value={preset}
            onChange={(event) => handlePresetChange(event.target.value as AcpAgentPreset)}
          >
            {ACP_AGENT_PRESETS.map((item) => (
              <option key={item.id} value={item.id}>
                {t.presets[item.id]}
              </option>
            ))}
          </select>
        </div>

        <div className="field">
          <label htmlFor={`${id}-command`}>{t.commandLabel}</label>
          <div className="field-row">
            <input
              id={`${id}-command`}
              type="text"
              className="text-input mono"
              value={command}
              placeholder={t.commandPlaceholder}
              onChange={(event) => {
                setCommand(event.target.value);
                setHandshakeResult(null);
                setHandshakeError(null);
              }}
            />
            <button type="submit" className="btn btn--secondary btn--sm">
              {t.saveConfig}
            </button>
          </div>
          <p className="field-hint">{t.guidance.presets[preset].setupHint}</p>
          <p className="field-hint">{t.commandHint}</p>
        </div>

        <div className="tutor-handshake-row">
          <button
            type="button"
            className="btn btn--primary btn--sm"
            disabled={testing}
            onClick={() => void handleTestHandshake()}
          >
            {testing ? t.testingHandshake : t.testHandshake}
          </button>
          {handshakeResult && (
            <span className="tutor-handshake-ok" role="status">
              {interpolate(t.handshakeOk, {
                agent: handshakeResult.agentVersion
                  ? `${resolvedAgentLabel} (${handshakeResult.agentVersion})`
                  : resolvedAgentLabel,
              })}
            </span>
          )}
        </div>

        {handshakeError && (
          <TutorErrorNotice
            error={handshakeError}
            fallbackPreset={preset}
            locale={settings.locale}
            messages={messages}
          />
        )}
      </form>

      <div className="field">
        <span className="tutor-field-label">{t.policyLabel}</span>
        <div
          className="tutor-policy-grid"
          role="radiogroup"
          aria-label={t.policyGroupLabel}
        >
          {(["guided_readonly", "strict_ask_all"] as const).map((policy) => {
            const active = settings.tutor.permissionPolicy === policy;
            return (
              <button
                key={policy}
                type="button"
                role="radio"
                aria-checked={active}
                className={`tutor-policy-option${active ? " is-active" : ""}`}
                onClick={() => handlePolicyChange(policy)}
              >
                <strong>{t.policies[policy].title}</strong>
                <span>{t.policies[policy].description}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="field">
        <span className="tutor-field-label">{t.styleLabel}</span>
        <div
          className="segmented tutor-style-segmented"
          role="group"
          aria-label={t.styleGroupLabel}
        >
          {(["socratic", "direct"] as const).map((style) => (
            <button
              key={style}
              type="button"
              className={`seg-btn${settings.tutor.teachingStyle === style ? " is-active" : ""}`}
              aria-pressed={settings.tutor.teachingStyle === style}
              onClick={() => handleStyleChange(style)}
            >
              {t.styles[style]}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
