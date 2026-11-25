import { lazy, Suspense, useEffect, useId, useState, type KeyboardEvent } from "react";

import { CloseIcon, TerminalIcon } from "../components/Icons";
import type { GlobalSettings, TerminalSummary } from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { WorkspaceDialog } from "../workspace/WorkspaceDialog";
import type { CommandWorkbench } from "./CommandWorkbench";

const TerminalViewport = lazy(() => import("./TerminalViewport"));

export function TerminalPanel({ workbench, state, messages, settings, disabled, onError }: {
  workbench: CommandWorkbench;
  state: ReturnType<CommandWorkbench["snapshot"]>;
  messages: Messages;
  settings: GlobalSettings;
  disabled: boolean;
  onError: (error: unknown) => void;
}) {
  const t = messages.execution;
  const [maximized, setMaximized] = useState(false);
  const [clearCount, setClearCount] = useState(0);
  const [errorContext, setErrorContext] = useState<string | null>(null);
  const panelId = useId();
  const active = state.terminals.find(({ id }) => id === state.active);
  const conflicted = active?.portConflict ? active : state.terminals.find(({ portConflict }) => portConflict);
  const service = conflicted && state.terminals.find((item) =>
    item.mode === "daemon" && item.running && item.port !== null && item.port === conflicted.port
  ) || conflicted;
  const isDaemon = active?.mode === "daemon";
  const busy = disabled || state.busy;
  const canRender = typeof ResizeObserver !== "undefined";

  function terminalName(item: TerminalSummary) {
    if (item.label) return item.label;
    const daemon = item.mode === "daemon";
    const group = state.terminals.filter(({ mode }) => (mode === "daemon") === daemon);
    const index = group.findIndex(({ id }) => id === item.id) + 1;
    return `${daemon ? t.daemon : t.terminal} ${index}`;
  }

  const activeId = active?.id;
  useEffect(() => {
    if (activeId === undefined || canRender) return;
    let cancelled = false;
    async function readStatus() {
      try {
        await workbench.read();
      } catch (error) {
        if (!cancelled) onError(error);
      }
    }
    void readStatus();
    const timer = setInterval(() => void readStatus(), 250);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [activeId, canRender, onError, workbench]);

  function handleTabKey(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const count = state.terminals.length;
    let next: number;
    switch (event.key) {
      case "ArrowRight": next = (index + 1) % count; break;
      case "ArrowLeft": next = (index + count - 1) % count; break;
      case "Home": next = 0; break;
      case "End": next = count - 1; break;
      default: return;
    }
    event.preventDefault();
    workbench.select(state.terminals[next].id);
    event.currentTarget.closest('[role="tablist"]')?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
  }

  async function askAI() {
    try {
      const { recentErrors } = await workbench.read(undefined, undefined, true);
      setErrorContext(recentErrors.join("\n"));
    } catch (error) {
      onError(error);
    }
  }

  return (
    <section className={`terminal-panel${active ? " terminal-panel--open" : ""}${active && maximized ? " terminal-panel--maximized" : ""}`}
      aria-label={t.terminals}>
      <div className="terminal-panel-toolbar">
        {active ? (
          <div className="terminal-tabs" role="tablist" aria-label={t.terminals}>
            {state.terminals.map((item, index) => (
              <button key={item.id} className="terminal-tab" type="button" role="tab"
                id={`${panelId}-tab-${item.id}`} aria-controls={panelId} aria-selected={item.id === active.id}
                tabIndex={item.id === active.id ? 0 : -1} title={item.cwd}
                onKeyDown={(event) => handleTabKey(event, index)}
                onClick={() => workbench.select(item.id)}>
                {item.mode === "daemon" && item.running && <span className="terminal-pulse" role="img" aria-label={t.running} />}
                {terminalName(item)}
              </button>
            ))}
          </div>
        ) : <span className="terminal-panel-title"><TerminalIcon size={14} />{t.terminal}</span>}
        <button className="btn btn--ghost btn--xs" type="button" disabled={busy} onClick={() => void workbench.open().catch(onError)}>
          <span aria-hidden="true">+</span> {t.newTerminal}
        </button>
        {active && <>
          <button className="btn btn--ghost btn--xs" type="button" onClick={() => setClearCount((count) => count + 1)}>{t.clear}</button>
          <button className="btn btn--ghost btn--xs" type="button" aria-pressed={maximized}
            onClick={() => setMaximized((value) => !value)}>{maximized ? t.restore : t.maximize}</button>
          <button className="btn btn--ghost btn--xs" type="button" disabled={busy} aria-label={t.close} title={t.close}
            onClick={() => void workbench.updateTerminal({ type: "closeTerminal", payload: workbench.target(active.id) }).catch(onError)}>
            <CloseIcon size={14} />
          </button>
        </>}
      </div>
      {active && (
        <div className="terminal-content" role="tabpanel" id={panelId} aria-labelledby={`${panelId}-tab-${active.id}`}>
          {isDaemon && <div className="terminal-process">
            <span>{t.daemon}</span>
            {active.port !== null && <span className="mono">{interpolate(t.port, { port: active.port })}</span>}
            <time className="mono" dateTime={active.startedAt} title={active.startedAt}>{new Date(active.startedAt).toLocaleTimeString(settings.locale)}</time>
            <button className="btn btn--ghost btn--xs" type="button" disabled={busy || !active.running} onClick={() =>
              void workbench.updateTerminal({ type: "stopTerminal", payload: workbench.target(active.id) }).catch(onError)
            }>{t.stop}</button>
            {active.commandId && <button className="btn btn--ghost btn--xs" type="button" disabled={busy} onClick={() => {
              if (active.commandId) void workbench.run({ commandId: active.commandId, restartTerminalId: active.id }).catch(onError);
            }}>{t.restart}</button>}
          </div>}
          {conflicted && service && (
            <div className="terminal-recovery" role="region" aria-label={t.portConflict}>
              <span>{t.portConflict}</span>
              <button className="btn btn--secondary btn--xs" type="button" onClick={() => workbench.select(service.id)}>{t.showService}</button>
              <button className="btn btn--ghost btn--xs" type="button" onClick={() => void askAI()}>{t.askAI}</button>
            </div>
          )}
          {canRender && <Suspense fallback={null}>
            <TerminalViewport key={active.id} workbench={workbench} id={active.id}
              settings={settings} clearCount={clearCount} label={terminalName(active)} onError={onError} />
          </Suspense>}
          {active.exitCode !== null && <div className="terminal-exit mono">{interpolate(t.exited, { code: active.exitCode })}</div>}
        </div>
      )}
      {errorContext !== null && (
        <WorkspaceDialog title={t.askAI} detail={<>{t.tutorUnavailable}{errorContext && <pre className="terminal-command-preview mono">{errorContext}</pre>}</>}
          cancel={messages.workspace.cancel} onCancel={() => setErrorContext(null)}>
          {errorContext && <button type="button" className="btn btn--secondary btn--sm" onClick={() => {
            if (navigator.clipboard) void navigator.clipboard.writeText(errorContext).catch(onError);
          }}>{t.copyErrors}</button>}
        </WorkspaceDialog>
      )}
    </section>
  );
}
