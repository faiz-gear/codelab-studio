import type { CommandBlock } from "../core/contracts";
import type { Messages } from "../i18n";
import { CopyIcon, TerminalIcon } from "./Icons";

export interface CommandBlockCardProps {
  command: CommandBlock;
  messages: Messages;
  onCopy: (text: string) => void;
}

export function CommandBlockCard({
  command,
  messages,
  onCopy,
}: CommandBlockCardProps) {
  const t = messages.workbench;
  const cwdLabel = command.cwd ?? t.cwdWorkspaceRoot;
  const isDaemon = command.mode === "daemon";

  return (
    <div
      className={`cmd-card${isDaemon ? " cmd-card--daemon" : ""}`}
      data-testid={`command-block-${command.id}`}
    >
      <div className="cmd-head">
        <span className="cmd-shell mono">
          <TerminalIcon size={12} />
          <span>{command.shell}</span>
        </span>
        <span className="cwd-pill" title={cwdLabel}>
          {cwdLabel}
        </span>
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
