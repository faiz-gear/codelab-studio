import { ShieldAlertIcon } from "../components/Icons";
import type { TutorPermissionRequest } from "../core/contracts";
import type { Messages } from "../i18n";

export interface TutorPermissionCardProps {
  permission: TutorPermissionRequest;
  messages: Messages;
  busy: boolean;
  onRespond: (requestId: string, allow: boolean) => void;
}

export function TutorPermissionCard({
  permission,
  messages,
  busy,
  onRespond,
}: TutorPermissionCardProps) {
  const t = messages.tutor;
  const isPending = permission.status === "pending";
  const isAutoAllowed = permission.status === "autoAllowed";
  const isBlockedSensitive = permission.status === "blockedSensitive";

  if (isAutoAllowed) {
    return (
      <div
        className="tutor-perm tutor-perm--auto"
        data-testid={`tutor-perm-${permission.id}`}
      >
        <span className="badge badge--neutral">{t.permissionKinds[permission.kind]}</span>
        <span className="tutor-perm-status">{t.permissionStatuses.autoAllowed}</span>
        {permission.targetPath && (
          <code className="mono tutor-perm-target">{permission.targetPath}</code>
        )}
      </div>
    );
  }

  const cardClass = isBlockedSensitive
    ? "tutor-perm tutor-perm--blocked"
    : isPending && permission.highRisk
      ? "tutor-perm tutor-perm--high-risk"
      : isPending
        ? "tutor-perm tutor-perm--pending"
        : "tutor-perm tutor-perm--done";

  return (
    <div
      className={cardClass}
      role={isPending ? "region" : undefined}
      aria-label={t.permissionTitle}
      data-testid={`tutor-perm-${permission.id}`}
      data-status={permission.status}
      data-high-risk={permission.highRisk ? "true" : "false"}
    >
      <div className="tutor-perm-head">
        <span className="tutor-perm-title">
          <ShieldAlertIcon size={14} />
          <span>{t.permissionTitle}</span>
        </span>
        <span className={`badge ${permission.highRisk && isPending ? "badge--danger" : "badge--neutral"}`}>
          {t.permissionKinds[permission.kind]}
        </span>
        {!isPending && (
          <span className="tutor-perm-status-badge">
            {t.permissionStatuses[permission.status]}
          </span>
        )}
      </div>

      {permission.outsideWorkspace && (
        <p className="tutor-perm-warning">{t.outsideWorkspaceWarning}</p>
      )}
      {permission.highRisk && (
        <p className="tutor-perm-danger" role="alert">
          {t.highRiskWarning}
        </p>
      )}
      {isBlockedSensitive && (
        <p className="tutor-perm-detail">{t.sensitiveBlockedDetail}</p>
      )}

      {(permission.command || permission.targetPath) && (
        <pre className="tutor-perm-code mono">
          {permission.command ?? permission.targetPath}
        </pre>
      )}

      {isPending && (
        <div className="tutor-perm-actions">
          <button
            type="button"
            className="btn btn--secondary btn--xs btn--pill"
            disabled={busy}
            onClick={() => onRespond(permission.id, false)}
          >
            {t.deny}
          </button>
          <button
            type="button"
            className={`btn btn--xs btn--pill ${permission.highRisk ? "btn--danger" : "btn--primary"}`}
            disabled={busy}
            onClick={() => onRespond(permission.id, true)}
          >
            {t.allow}
          </button>
        </div>
      )}
    </div>
  );
}
