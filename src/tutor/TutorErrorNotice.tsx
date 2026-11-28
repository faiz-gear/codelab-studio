import {
  ACP_AGENT_PRESETS,
  type AcpAgentPreset,
  type DomainError,
  type Locale,
} from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";

const KNOWN_PRESETS = new Set<string>(ACP_AGENT_PRESETS.map((item) => item.id));

function resolvePreset(
  error: DomainError,
  fallbackPreset: AcpAgentPreset
): AcpAgentPreset {
  const candidate = error.details?.preset;
  if (candidate && KNOWN_PRESETS.has(candidate)) {
    return candidate as AcpAgentPreset;
  }
  return fallbackPreset;
}

export interface TutorErrorNoticeProps {
  error: DomainError;
  fallbackPreset: AcpAgentPreset;
  locale: Locale;
  messages: Messages;
  onOpenSettings?: (trigger: HTMLElement) => void;
}

export function TutorErrorNotice({
  error,
  fallbackPreset,
  locale,
  messages,
  onOpenSettings,
}: TutorErrorNoticeProps) {
  const preset = resolvePreset(error, fallbackPreset);
  const guidance = messages.tutor.guidance;
  const presetGuide = guidance.presets[preset];
  const isMissingAdapter = error.code === "ACP_ADAPTER_NOT_FOUND";
  const isAuthRequired = error.code === "ACP_AUTH_REQUIRED";

  return (
    <div className="inline-alert inline-alert--danger tutor-error-notice" role="alert">
      <p className="tutor-error-summary">{formatDomainError(error, locale)}</p>
      {isMissingAdapter && (
        <div className="tutor-guidance-block">
          <p className="tutor-guidance-detail">{presetGuide.installSummary}</p>
          {presetGuide.installCommand && (
            <code className="tutor-guidance-cmd mono">{presetGuide.installCommand}</code>
          )}
          {onOpenSettings && (
            <div className="tutor-guidance-actions">
              <button
                type="button"
                className="btn btn--secondary btn--xs"
                onClick={(event) => onOpenSettings(event.currentTarget)}
              >
                {guidance.openSettingsAction}
              </button>
            </div>
          )}
        </div>
      )}
      {isAuthRequired && (
        <div className="tutor-guidance-block">
          <p className="tutor-guidance-detail">{presetGuide.authSummary}</p>
          {presetGuide.loginCommand && (
            <code className="tutor-guidance-cmd mono">{presetGuide.loginCommand}</code>
          )}
        </div>
      )}
    </div>
  );
}
