import type { CommandBlock } from "../core/contracts";
import type { Messages } from "../i18n";
import { CopyIcon, PlayIcon, TerminalIcon } from "./Icons";

export interface CommandBlockCardProps {
  command: CommandBlock;
  messages: Messages;
  onCopy: (text: string) => void;
  override?: string;
  disabled?: boolean;
  onRun?: () => void;
  onChangeCwd?: () => void;
}

export function CommandBlockCard({
  command,
  messages,
  onCopy, override, disabled, onRun, onChangeCwd,
}: CommandBlockCardProps) {
  const t = messages.workbench;
  const cwd = override ?? command.cwd;
  const cwdLabel = cwd && cwd !== "." ? cwd : t.cwdWorkspaceRoot;
  const execution = messages.execution;
  const isDaemon = command.mode === "daemon";

  return (
    <div
      className={`cmd-card${isDaemon ? " cmd-card--daemon" : ""}${onRun ? " cmd-card--runnable" : ""}`}
      data-testid={`command-block-${command.id}`}
    >
      <div className="cmd-head">
        <span className="cmd-shell mono">
          <TerminalIcon size={12} />
          <span>{command.shell}</span>
        </span>
        {onChangeCwd ? (
          <button type="button" className="cwd-pill cwd-pill--editable" title={cwdLabel}
            aria-label={execution.changeCwd} disabled={disabled} onClick={onChangeCwd}>{cwdLabel}</button>
        ) : <span className="cwd-pill" title={cwdLabel}>{cwdLabel}</span>}
        <span
          className={`badge ${isDaemon ? "badge--info" : "badge--neutral"}`}
        >
          {isDaemon ? t.commandModeDaemon : t.commandModeOneshot}
        </span>
        {command.label ? (
          <span className="cmd-label" title={command.label}>
            {command.label}
          </span>
        ) : null}
        <div className="cmd-actions">
          {onRun && (
            <button type="button" className="btn btn--secondary btn--xs" disabled={disabled}
              aria-label={execution.run} title={execution.run} onClick={onRun}><PlayIcon size={12} />{execution.run}</button>
          )}
          <button
            type="button"
            className="btn btn--ghost btn--xs"
            aria-label={t.copyCommand}
            title={t.copyCommand}
            onClick={() => onCopy(command.command)}
          >
            <CopyIcon size={12} />
            <span>{t.copyCommand}</span>
          </button>
        </div>
      </div>
      <pre className="cmd-code">
        <code>{command.command}</code>
      </pre>
    </div>
  );
}
