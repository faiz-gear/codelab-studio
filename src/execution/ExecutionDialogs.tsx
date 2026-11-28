import { useState } from "react";

import type { CommandBlock, CommandConfirmation, Locale } from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";
import { domainError } from "../workspace/editorSession";
import { WorkspaceDialog } from "../workspace/WorkspaceDialog";

export function CommandConfirmationDialog({ detail, messages, onCancel, onConfirm }: {
  detail: CommandConfirmation;
  messages: Messages;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const t = messages.execution;
  return (
    <WorkspaceDialog title={t.confirmTitle} className="execution-danger-dialog" detail={<>
      {t.confirmDetail}
      <span className="workspace-dialog-path mono">{detail.cwd}</span>
      <pre className="terminal-command-preview mono">{detail.command}</pre>
    </>} cancel={messages.workspace.cancel} onCancel={onCancel}>
      <button className="btn btn--secondary btn--sm execution-danger-button" type="button" onClick={onConfirm}>{t.confirmAction}</button>
    </WorkspaceDialog>
  );
}

export function CommandCwdDialog({ command, override, messages, locale, onCancel, onSave }: {
  command: CommandBlock;
  override: string | undefined;
  messages: Messages;
  onCancel: () => void;
  onSave: (value: string | null) => Promise<void>;
  locale: Locale;
}) {
  const [value, setValue] = useState(override ?? command.cwd ?? ".");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const t = messages.execution;
  function save(path: string | null) {
    setSaving(true);
    setError(null);
    void onSave(path).then(onCancel, (failure: unknown) => {
      setError(formatDomainError(domainError(failure), locale));
    }).finally(() => setSaving(false));
  }
  return (
    <WorkspaceDialog title={t.changeCwd} detail={<>
      {t.cwdHint}
      <label className="terminal-cwd-field">
        <span>{t.cwd}</span>
        <input className="text-input mono" value={value} disabled={saving} onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => { if (event.key === "Enter" && value.trim()) save(value.trim()); }} />
      </label>
      {error && <p className="terminal-cwd-error" role="alert">{error}</p>}
    </>} cancel={messages.workspace.cancel} busy={saving} onCancel={onCancel}>
      {override !== undefined && <button className="btn btn--ghost btn--sm" type="button" disabled={saving} onClick={() => save(null)}>{t.resetCwd}</button>}
      <button className="btn btn--primary btn--sm" type="button" disabled={saving || !value.trim()} onClick={() => save(value.trim())}>{messages.workspace.save}</button>
    </WorkspaceDialog>
  );
}
