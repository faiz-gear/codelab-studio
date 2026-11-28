import { useId, useRef, useState, type KeyboardEvent } from "react";
import { Select } from "../components/Select";
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
import { formatDomainError, interpolate, type Messages } from "../i18n";
import { domainError } from "../workspace/editorSession";

const PERMISSION_POLICIES: readonly PermissionPolicy[] = [
  "guided_readonly",
  "strict_ask_all",
] as const;

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
  const policyRefs = useRef<Record<PermissionPolicy, HTMLButtonElement | null>>({
    guided_readonly: null,
    strict_ask_all: null,
  });

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

  const handlePolicyKeyDown = (
    event: KeyboardEvent<HTMLButtonElement>,
    current: PermissionPolicy
  ) => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
      return;
    }
    event.preventDefault();
    const index = PERMISSION_POLICIES.indexOf(current);
    const next =
      PERMISSION_POLICIES[(index + 1) % PERMISSION_POLICIES.length];
    handlePolicyChange(next);
    policyRefs.current[next]?.focus();
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

  const activePresetMeta = ACP_AGENT_PRESETS.find((item) => item.id === preset);
  const resolvedAgentLabel =
    handshakeResult?.agentName ??
    activePresetMeta?.label ??
    t.presets[preset];

  const handshakeBadgeTone = testing
    ? "neutral"
    : handshakeResult
      ? "success"
      : handshakeError
        ? "danger"
        : "neutral";
  const handshakeBadgeText = testing
    ? t.handshakeStatusTesting
    : handshakeResult
      ? t.handshakeStatusOk
      : handshakeError
        ? t.handshakeStatusFailed
        : t.handshakeStatusIdle;

  return (
    <div className="tutor-settings" data-testid="tutor-settings-panel">
      <div className="tutor-settings-banner">
        <div className="tutor-settings-banner-head">
          <span className="badge badge--ai">{t.protocolBadge}</span>
          <span className="badge badge--neutral">{resolvedAgentLabel}</span>
        </div>
        <p className="tutor-settings-intro">{t.settingsIntro}</p>
      </div>

      {/* Task Group 1: Agent preset & launch command */}
      <section className="settings-card" aria-labelledby={`${id}-connection`}>
        <div className="settings-card-head">
          <h3 id={`${id}-connection`}>{t.connectionSectionTitle}</h3>
          {command.trim() ? (
            <span className="badge badge--neutral mono tutor-cmd-preview" title={command.trim()}>
              {command.trim()}
            </span>
          ) : null}
        </div>
        <form className="tutor-settings-form" onSubmit={(event) => void handleSaveCommand(event)}>
          <div className="field">
            <label htmlFor={`${id}-preset`}>{t.presetLabel}</label>
            <Select<AcpAgentPreset>
              id={`${id}-preset`}
              value={preset}
              options={ACP_AGENT_PRESETS.map((item) => ({
                value: item.id,
                label: t.presets[item.id],
              }))}
              onChange={handlePresetChange}
            />
          </div>

          <div className="field">
            <label htmlFor={`${id}-command`}>{t.commandLabel}</label>
            <div className="field-row tutor-command-row">
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
            <p className="field-hint">{t.commandHint}</p>
          </div>
        </form>
      </section>

      {/* Task Group 2: Handshake test & live feedback */}
      <section className="settings-card" aria-labelledby={`${id}-handshake`}>
        <div className="settings-card-head">
          <h3 id={`${id}-handshake`}>{t.handshakeSectionTitle}</h3>
          <span className={`badge badge--${handshakeBadgeTone}`}>{handshakeBadgeText}</span>
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
          {handshakeResult ? (
            <span className="tutor-handshake-ok" role="status">
              {interpolate(t.handshakeOk, {
                agent: handshakeResult.agentVersion
                  ? `${resolvedAgentLabel} (${handshakeResult.agentVersion})`
                  : resolvedAgentLabel,
              })}
            </span>
          ) : !handshakeError ? (
            <p className="field-hint tutor-handshake-hint">{t.handshakeIdleHint}</p>
          ) : null}
        </div>
        {handshakeError && (
          <div className="inline-alert inline-alert--danger" role="alert">
            {formatDomainError(handshakeError, settings.locale)}
          </div>
        )}
      </section>

      {/* Task Group 3: Permission policy (ADR-0009) */}
      <section className="settings-card" aria-labelledby={`${id}-policy`}>
        <div className="settings-card-head">
          <h3 id={`${id}-policy`}>{t.policyLabel}</h3>
          <span className="badge badge--neutral">
            {t.policies[settings.tutor.permissionPolicy].title}
          </span>
        </div>
        <p className="field-hint">{t.policySectionHint}</p>
        <div
          className="tutor-policy-grid"
          role="radiogroup"
          aria-label={t.policyGroupLabel}
        >
          {PERMISSION_POLICIES.map((policy) => {
            const active = settings.tutor.permissionPolicy === policy;
            return (
              <button
                key={policy}
                ref={(el) => {
                  policyRefs.current[policy] = el;
                }}
                type="button"
                role="radio"
                aria-checked={active}
                tabIndex={active ? 0 : -1}
                className={`tutor-policy-option${active ? " is-active" : ""}`}
                onClick={() => handlePolicyChange(policy)}
                onKeyDown={(event) => handlePolicyKeyDown(event, policy)}
              >
                <span className="tutor-policy-option-head">
                  <span className="tutor-radio-dot" aria-hidden="true" />
                  <strong>{t.policies[policy].title}</strong>
                </span>
                <span className="tutor-policy-option-desc">{t.policies[policy].description}</span>
              </button>
            );
          })}
        </div>
      </section>

      {/* Task Group 4: Teaching style */}
      <section className="settings-card" aria-labelledby={`${id}-style`}>
        <div className="settings-card-head">
          <h3 id={`${id}-style`}>{t.styleLabel}</h3>
          <span className="badge badge--neutral">{t.styles[settings.tutor.teachingStyle]}</span>
        </div>
        <p className="field-hint">{t.styleSectionHint}</p>
        <div
          className="seg tutor-style-segmented"
          role="group"
          aria-label={t.styleGroupLabel}
        >
          {(["socratic", "direct"] as const).map((style) => (
            <button
              key={style}
              type="button"
              aria-pressed={settings.tutor.teachingStyle === style}
              onClick={() => handleStyleChange(style)}
            >
              {t.styles[style]}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}
