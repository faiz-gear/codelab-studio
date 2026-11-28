import type {
  CourseLanguage,
  DomainError,
  InstallProgress,
  LanguagePreflight,
  Locale,
  PreflightCommand,
  PreflightReport,
  ProbeStatus,
  ToolCheck,
} from "../core/contracts";
import { formatDomainError, interpolate, type Messages } from "../i18n";

export interface RuntimePanelProps {
  report: PreflightReport | null;
  error: DomainError | null;
  messages: Messages;
  locale: Locale;
  onCommand: (command: PreflightCommand) => void;
}

type RuntimeMessages = Messages["runtime"];

function Status({
  status,
  copy,
}: {
  status: ProbeStatus;
  copy: RuntimeMessages;
}) {
  const tone =
    status === "ok" ? "success" : status === "warn" ? "warning" : "neutral";
  return <span className={`badge badge--${tone}`}>{copy.status[status]}</span>;
}

function ToolStatus({
  check,
  copy,
}: {
  check: ToolCheck;
  copy: RuntimeMessages;
}) {
  return (
    <span className="runtime-tool">
      <code>{copy.toolNames[check.tool]}</code>
      {check.value && <code className="mono runtime-value">{check.value}</code>}
      <Status status={check.status} copy={copy} />
      {check.problem && <span>{copy.problems[check.problem]}</span>}
    </span>
  );
}

function Installation({
  progress,
  language,
  copy,
  locale,
}: {
  progress: InstallProgress;
  language: CourseLanguage;
  copy: RuntimeMessages;
  locale: Locale;
}) {
  switch (progress.phase) {
    case "failed":
      return (
        <p role="alert" className="runtime-error">
          {formatDomainError(progress.error, locale)}
        </p>
      );
    case "installed":
      return (
        <p role="status" className="runtime-installed">
          {copy.installed}
        </p>
      );
    case "downloading": {
      const total = progress.totalBytes;
      const value = total ? progress.receivedBytes : undefined;
      return (
        <div className="runtime-download">
          <p>
            {interpolate(copy.downloading, { artifact: progress.artifact })}
            {total ? (
              <span className="mono">
                {Math.round((progress.receivedBytes * 100) / total)}%
              </span>
            ) : null}
          </p>
          <progress
            aria-label={interpolate(copy.progressLabel, { language })}
            value={value}
            max={total || 1}
          />
        </div>
      );
    }
    case "preparing":
    case "extracting":
      return (
        <p role="status" className="runtime-detail">
          {progress.phase === "preparing"
            ? copy.preparing
            : interpolate(copy.extracting, { artifact: progress.artifact })}
        </p>
      );
  }
}

function RuntimeRow({
  row,
  checked,
  expanded,
  copy,
  locale,
  onCommand,
}: {
  row: LanguagePreflight;
  checked: boolean;
  expanded: boolean;
  copy: RuntimeMessages;
  locale: Locale;
  onCommand: RuntimePanelProps["onCommand"];
}) {
  const selection = row.selected;
  const selectedManaged = selection?.source === "managed" ? row.managed : null;
  const selectedSystem =
    selection?.source === "system"
      ? row.system.find((check) => check.tool === selection.tool)
      : null;
  const failed = row.install?.phase === "failed";

  return (
    <li className="runtime-row">
      <div className="runtime-row-head">
        <h3>{row.language}</h3>
        {(checked || row.selected) && (
          <Status status={row.status} copy={copy} />
        )}
        <span className="runtime-selection">
          {selectedManaged && (
            <>
              {copy.managed} <code>{selectedManaged.version}</code>
            </>
          )}
          {selectedSystem && (
            <>
              {copy.system}{" "}
              <code>
                {copy.toolNames[selectedSystem.tool]} {selectedSystem.value}
              </code>
            </>
          )}
        </span>
        {row.canInstall && (
          <button
            type="button"
            className="btn btn--secondary btn--xs"
            aria-label={interpolate(failed ? copy.retryFor : copy.installFor, {
              language: row.language,
            })}
            onClick={() =>
              onCommand({
                type: "installManagedRuntime",
                payload: { language: row.language },
              })
            }
          >
            {failed ? copy.retry : copy.install}
          </button>
        )}
      </div>

      {(expanded || !row.selected) && row.system.length > 0 && (
        <div className="runtime-tools">
          {row.system.map((check) => (
            <ToolStatus key={check.tool} check={check} copy={copy} />
          ))}
        </div>
      )}
      {row.lsp && (
        <div className="runtime-lsp">
          <span className="runtime-detail">{copy.optionalLsp}</span>
          <ToolStatus check={row.lsp} copy={copy} />
          {row.lsp.status !== "ok" && (
            <p className="runtime-detail">{copy.lspGuidance[row.lsp.tool]}</p>
          )}
        </div>
      )}

      {row.install && (
        <Installation
          progress={row.install}
          language={row.language}
          copy={copy}
          locale={locale}
        />
      )}

      {expanded && (
        <label className="runtime-preference">
          <input
            type="checkbox"
            className="arc-checkbox"
            checked={row.preferManaged}
            aria-label={interpolate(copy.preferFor, { language: row.language })}
            onChange={(event) =>
              onCommand({
                type: "setRuntimePreference",
                payload: {
                  language: row.language,
                  preferManaged: event.currentTarget.checked,
                },
              })
            }
          />
          <span>{copy.preferManaged}</span>
        </label>
      )}
    </li>
  );
}

export function RuntimePanel({
  report,
  error,
  messages,
  locale,
  onCommand,
  expanded = false,
}: RuntimePanelProps & { expanded?: boolean }) {
  const copy = messages.runtime;
  const tools = report?.goProxy
    ? [...report.tools, report.goProxy]
    : report?.tools;
  return (
    <div className="runtime-panel">
      <div className="runtime-toolbar">
        {report?.probing || (!report && !error) ? (
          <p role="status">{copy.checking}</p>
        ) : null}
        <button
          type="button"
          className="btn btn--ghost btn--xs"
          disabled={report?.probing}
          onClick={() => onCommand({ type: "refreshPreflight" })}
        >
          {copy.refresh}
        </button>
      </div>
      {error && (
        <p role="alert" className="runtime-error">
          {formatDomainError(error, locale)}
        </p>
      )}
      {report && (
        <>
          <ul className="runtime-list">
            {report.languages.map((row) => (
              <RuntimeRow
                key={row.language}
                row={row}
                checked={report.checked}
                expanded={expanded}
                copy={copy}
                locale={locale}
                onCommand={onCommand}
              />
            ))}
          </ul>
          <div className="runtime-tools">
            {tools?.map((check) => (
              <ToolStatus key={check.tool} check={check} copy={copy} />
            ))}
          </div>
          {expanded && (
            <p className="runtime-detail runtime-install-dir">
              {copy.installDir}{" "}
              <code className="mono">{report.installDir}</code>
            </p>
          )}
        </>
      )}
    </div>
  );
}
