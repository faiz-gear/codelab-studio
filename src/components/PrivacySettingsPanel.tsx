import { useEffect, useId, useRef, useState } from "react";
import { expectResponse, type CoreClient } from "../core/client";
import {
  METRIC_EVENT_NAMES,
  type DomainError,
  type Locale,
  type TrustEntry,
  type TrustSource,
  type WorkspaceMetrics,
  type WorkspaceSummary,
} from "../core/contracts";
import { formatDomainError, formatRelativeTime, interpolate, type Messages } from "../i18n";
import { domainError } from "../workspace/editorSession";
import { RefreshIcon } from "./Icons";

export interface PrivacySettingsPanelProps {
  client: CoreClient;
  locale: Locale;
  messages: Messages;
  trustEntries: TrustEntry[];
  activeWorkspace: WorkspaceSummary | null;
  activeCourseTrusted: boolean;
  onRevokeTrustEntry: (source: TrustSource) => Promise<void>;
  onClearTutorHistory: (workspaceId: number) => Promise<void>;
  onResetWorkspaceProgress: (workspaceId: number) => Promise<void>;
}

export function PrivacySettingsPanel({
  client,
  locale,
  messages,
  trustEntries,
  activeWorkspace,
  activeCourseTrusted,
  onRevokeTrustEntry,
  onClearTutorHistory,
  onResetWorkspaceProgress,
}: PrivacySettingsPanelProps) {
  const id = useId();
  const p = messages.settings.privacy;
  const [metricsState, setMetricsState] = useState<{
    workspaceId: number;
    requestId: number;
    metrics: WorkspaceMetrics | null;
    error: DomainError | null;
  } | null>(null);
  const [refreshNonce, setRefreshNonce] = useState(0);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmingReset, setConfirmingReset] = useState(false);
  const cancelResetRef = useRef<HTMLButtonElement | null>(null);

  const canManageWorkspace = Boolean(activeWorkspace && activeCourseTrusted);
  const workspaceId = canManageWorkspace ? activeWorkspace!.id : null;

  useEffect(() => {
    if (workspaceId === null) return;
    let active = true;
    const requestId = refreshNonce;
    void client
      .dispatch({
        type: "getWorkspaceMetrics",
        payload: { workspaceId },
      })
      .then((response) => {
        if (!active) return;
        setMetricsState({
          workspaceId,
          requestId,
          metrics: expectResponse(response, "workspaceMetrics").payload,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (!active) return;
        setMetricsState({
          workspaceId,
          requestId,
          metrics: null,
          error: domainError(error),
        });
      });
    return () => {
      active = false;
    };
  }, [client, refreshNonce, workspaceId]);

  const resolvedState =
    workspaceId !== null && metricsState?.workspaceId === workspaceId
      ? metricsState
      : null;
  const metrics = resolvedState?.metrics ?? null;
  const metricsError = resolvedState?.error ?? null;
  const metricsLoading =
    workspaceId !== null &&
    (resolvedState === null || resolvedState.requestId !== refreshNonce);

  useEffect(() => {
    if (confirmingReset) {
      cancelResetRef.current?.focus();
    }
  }, [confirmingReset]);

  const totalMetricCount = metrics
    ? METRIC_EVENT_NAMES.reduce((sum, name) => sum + (metrics.counts[name] ?? 0), 0)
    : 0;

  return (
    <div className="settings-section-stack" data-testid="privacy-settings-panel">
      <div className="settings-privacy-banner">
        <div className="settings-privacy-banner-head">
          <span className="badge badge--success">{p.zeroTelemetryBadge}</span>
          <strong>{p.bannerTitle}</strong>
        </div>
        <p>{p.bannerBody}</p>
      </div>

      <section className="settings-card" aria-labelledby={`${id}-trust`}>
        <div className="settings-card-head">
          <h3 id={`${id}-trust`}>{p.trustHeading}</h3>
          <span className="badge badge--neutral mono">{trustEntries.length}</span>
        </div>
        <p className="field-hint">{p.trustHint}</p>
        {trustEntries.length === 0 ? (
          <p className="settings-empty-note">{p.trustEmpty}</p>
        ) : (
          <ul className="settings-trust-list">
            {trustEntries.map((entry) => {
              const key = `${entry.sourceKind}:${entry.normalizedSource}`;
              const isBusy = busyAction === `trust:${key}`;
              return (
                <li key={key} className="settings-trust-item">
                  <div className="settings-trust-meta">
                    <span className="badge badge--neutral">
                      {entry.sourceKind === "gitRepo" ? p.trustKindGit : p.trustKindLocal}
                    </span>
                    <span className="mono settings-trust-path" title={entry.normalizedSource}>
                      {entry.normalizedSource}
                    </span>
                    <span className="settings-trust-time">
                      {formatRelativeTime(entry.trustedAt, locale)}
                    </span>
                  </div>
                  <button
                    type="button"
                    className="btn btn--secondary btn--xs"
                    disabled={busyAction !== null}
                    onClick={async () => {
                      setBusyAction(`trust:${key}`);
                      try {
                        await onRevokeTrustEntry({
                          sourceKind: entry.sourceKind,
                          source: entry.normalizedSource,
                        });
                      } finally {
                        setBusyAction(null);
                      }
                    }}
                  >
                    {isBusy ? `${p.revokeTrust}…` : p.revokeTrust}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section className="settings-card" aria-labelledby={`${id}-workspace-data`}>
        <div className="settings-card-head">
          <h3 id={`${id}-workspace-data`}>{p.workspaceDataHeading}</h3>
          {activeWorkspace && (
            <span className="mono settings-workspace-chip" title={activeWorkspace.root}>
              {activeWorkspace.root}
            </span>
          )}
        </div>
        <p className="field-hint">{p.workspaceDataHint}</p>
        {!activeWorkspace ? (
          <p className="settings-empty-note">{p.noActiveWorkspaceHint}</p>
        ) : !activeCourseTrusted ? (
          <p className="settings-empty-note">{p.untrustedWorkspaceHint}</p>
        ) : (
          <div className="settings-danger-actions">
            <div className="settings-action-row">
              <div className="settings-action-copy">
                <strong>{p.clearTutorTitle}</strong>
                <span>{p.clearTutorHint}</span>
              </div>
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                disabled={busyAction !== null}
                onClick={async () => {
                  setBusyAction("clearTutor");
                  try {
                    await onClearTutorHistory(activeWorkspace.id);
                  } finally {
                    setBusyAction(null);
                  }
                }}
              >
                {p.clearTutorAction}
              </button>
            </div>

            <div className="settings-action-row">
              <div className="settings-action-copy">
                <strong>{p.resetProgressTitle}</strong>
                <span>{p.resetProgressHint}</span>
              </div>
              {confirmingReset ? (
                <div className="settings-inline-confirm">
                  <button
                    ref={cancelResetRef}
                    type="button"
                    className="btn btn--ghost btn--sm"
                    disabled={busyAction !== null}
                    onClick={() => setConfirmingReset(false)}
                  >
                    {messages.workspace.cancel}
                  </button>
                  <button
                    type="button"
                    className="btn btn--danger btn--sm"
                    disabled={busyAction !== null}
                    onClick={async () => {
                      setBusyAction("resetProgress");
                      try {
                        await onResetWorkspaceProgress(activeWorkspace.id);
                        setConfirmingReset(false);
                      } finally {
                        setBusyAction(null);
                      }
                    }}
                  >
                    {p.resetProgressConfirm}
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  className="btn btn--secondary btn--sm"
                  disabled={busyAction !== null}
                  onClick={() => setConfirmingReset(true)}
                >
                  {p.resetProgressAction}
                </button>
              )}
            </div>
          </div>
        )}
      </section>

      <section className="settings-card" aria-labelledby={`${id}-metrics`}>
        <div className="settings-card-head">
          <h3 id={`${id}-metrics`}>{p.metricsHeading}</h3>
          <div className="settings-metrics-head-right">
            <span className="badge badge--neutral mono">{p.metricsPathLabel}</span>
            {canManageWorkspace && (
              <button
                type="button"
                className="btn btn--ghost btn--xs"
                disabled={metricsLoading}
                onClick={() => setRefreshNonce((n) => n + 1)}
              >
                <RefreshIcon size={12} />
                <span>{p.metricsRefresh}</span>
              </button>
            )}
          </div>
        </div>
        <p className="field-hint">{p.metricsHint}</p>
        {!canManageWorkspace ? (
          <p className="settings-empty-note">{p.metricsEmpty}</p>
        ) : metricsLoading && !metrics ? (
          <p className="settings-empty-note" role="status">
            {p.metricsLoading}
          </p>
        ) : metricsError ? (
          <div className="inline-alert inline-alert--danger" role="alert">
            {formatDomainError(metricsError, locale)}
          </div>
        ) : (
          <>
            <p className="settings-metrics-total mono">
              {interpolate(p.metricsTotalLabel, { count: totalMetricCount })}
            </p>
            <dl className="settings-metrics-grid" data-testid="local-metrics-grid">
              {METRIC_EVENT_NAMES.map((name) => (
                <div key={name} className="settings-metric-cell">
                  <dt title={name}>{p.metricLabels[name]}</dt>
                  <dd className="mono">{metrics?.counts[name] ?? 0}</dd>
                </div>
              ))}
            </dl>
          </>
        )}
      </section>
    </div>
  );
}
