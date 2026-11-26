import { lazy, Suspense, useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent } from "react";
import { CloseIcon, FolderIcon } from "../components/Icons";
import type { ExternalIde, GlobalSettings } from "../core/contracts";
import { formatDomainError, interpolate, type Messages } from "../i18n";
import { editorText, type EditorTab, type WorkspaceEditorSession } from "./editorSession";
import { WorkspaceComparisonTrigger } from "./WorkspaceComparison";
import { WorkspaceDialog } from "./WorkspaceDialog";
import { WorkspaceTree } from "./WorkspaceTree";

const MonacoText = lazy(() => import("./MonacoSurface").then((module) => ({ default: module.MonacoTextSurface })));
const MonacoDiff = lazy(() => import("./MonacoSurface").then((module) => ({ default: module.MonacoDiffSurface })));

const IDES: Record<ExternalIde, string> = {
  vsCode: "VS Code",
  cursor: "Cursor",
  goLand: "GoLand",
  intelliJIdea: "IntelliJ IDEA",
};

export function WorkspaceEditor({
  session, settings, messages, onChangeWorkspace, onRevokeTrust, onOpenRuntimeSettings, switching, workspaceActionLabel, workspaceLabel, tutorOpen = false,
}: {
  session: WorkspaceEditorSession;
  settings: GlobalSettings;
  messages: Messages;
  onChangeWorkspace: () => void;
  workspaceActionLabel?: string;
  workspaceLabel?: string;
  onRevokeTrust?: () => void;
  onOpenRuntimeSettings?: (trigger: HTMLElement) => void;
  switching: boolean;
  tutorOpen?: boolean;
}) {
  const state = useSyncExternalStore(session.subscribe, session.snapshot);
  const { status: languageStatus } = useSyncExternalStore(session.language.subscribe, session.language.snapshot);
  const conflictBanner = useRef<HTMLDivElement>(null);
  const { executionConflict } = state;
  useEffect(() => {
    if (executionConflict > 0) conflictBanner.current?.focus();
  }, [executionConflict]);
  const [treeOpen, setTreeOpen] = useState(false);
  const [lastTutorOpen, setLastTutorOpen] = useState(tutorOpen);
  if (tutorOpen !== lastTutorOpen) {
    setLastTutorOpen(tutorOpen);
    // At 1280px-1360px, collapse the file tree when the docked AI drawer opens so the editor keeps ample width.
    if (
      tutorOpen &&
      typeof window !== "undefined" &&
      window.innerWidth >= 1280 &&
      window.innerWidth <= 1360
    ) {
      setTreeOpen(false);
    }
  }
  const [ide, setIde] = useState<ExternalIde>("vsCode");
  const [fileForm, setFileForm] = useState<{ rename: string | null; value: string } | null>(null);
  const [closingFile, setClosingFile] = useState<string | null>(null);
  const [comparison, setComparison] = useState<{ left: string; right: string | null } | null>(null);
  const t = messages.workspace;
  const active = state.tabs.find(({ file }) => file.path === state.activePath);
  const path = active?.file.path ?? null;
  const conflicted = active?.conflict ? active : state.tabs.find((tab) => tab.conflict);
  const dirty = state.tabs.some((tab) => tab.draft !== null);
  const text = active ? editorText(active) : null;
  const busy = state.busy || switching;
  const leftTab = state.tabs.find(({ file }) => file.path === comparison?.left);
  const rightTab = state.tabs.find(({ file }) => file.path === comparison?.right);
  const left = leftTab ? editorText(leftTab) : null;
  const right = rightTab ? editorText(rightTab) : null;
  const comparing = Boolean(leftTab && rightTab && comparison?.right);
  const pickingComparison = Boolean(leftTab && !comparison?.right);
  const label = interpolate(t.editFile, { path: path ?? "" });
  const useMonaco = typeof ResizeObserver !== "undefined";

  useEffect(() => {
    const handleSave = (event: globalThis.KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.key.toLowerCase() !== "s") return;
      event.preventDefault();
      if (!switching && !closingFile && !document.querySelector("dialog[open]")) {
        void session.save();
      }
    };
    window.addEventListener("keydown", handleSave, true);
    return () => window.removeEventListener("keydown", handleSave, true);
  }, [closingFile, session, switching]);

  function chooseComparison(file: EditorTab) {
    if (pickingComparison && comparison && comparison.left !== file.file.path) {
      setComparison({ ...comparison, right: file.file.path });
    }
  }

  async function openFile(filePath: string) {
    const selected = await session.open(filePath);
    const tab = session.snapshot().tabs.find(({ file }) => file.path === selected);
    if (tab) chooseComparison(tab);
  }

  function selectTab(tab: EditorTab) {
    if (!pickingComparison) setComparison(null);
    session.select(tab.file.path);
    chooseComparison(tab);
  }

  function closeTab(filePath: string) {
    if (comparison?.left === filePath || comparison?.right === filePath) setComparison(null);
    session.close(filePath);
    setClosingFile(null);
  }

  function handleTabKey(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const { length } = state.tabs;
    const next = event.key === "ArrowLeft" ? (index - 1 + length) % length
      : event.key === "ArrowRight" ? (index + 1) % length
      : event.key === "Home" ? 0 : event.key === "End" ? length - 1 : null;
    if (next === null) return;
    event.preventDefault();
    selectTab(state.tabs[next]);
    event.currentTarget.closest('[role="tablist"]')?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
  }

  return (
    <div className="workspace-editor">
      <div className="workspace-location">
        {workspaceLabel && <span className="badge badge--neutral">{workspaceLabel}</span>}
        <span className="mono workspace-root" title={session.workspace.root}>{session.workspace.root}</span>
        <button type="button" className="btn btn--ghost btn--xs" disabled={busy} onClick={onChangeWorkspace}>{workspaceActionLabel ?? t.change}</button>
        {onRevokeTrust && <button type="button" className="btn btn--ghost btn--xs" disabled={busy} onClick={onRevokeTrust}>{t.revokeTrust}</button>}
      </div>
      <div className="workspace-controls">
        <button type="button" className="btn btn--ghost btn--xs" aria-expanded={treeOpen} onClick={() => setTreeOpen(!treeOpen)}>
          <FolderIcon />{t.files}
        </button>
        <button type="button" className="btn btn--ghost btn--xs" disabled={busy} onClick={() => setFileForm({ rename: null, value: "" })}>{t.newFile}</button>
        <button type="button" className="btn btn--ghost btn--xs" disabled={!active || active.file.kind === "missing" || !!active.unavailable || busy}
          onClick={() => path && setFileForm({ rename: path, value: path })}>{t.rename}</button>
        <button type="button" className="btn btn--ghost btn--xs" disabled={text === null || busy}
          aria-pressed={pickingComparison || comparing}
          onClick={() => {
            if (pickingComparison || comparing) setComparison(null);
            else if (path) {
              setComparison({ left: path, right: null });
              setTreeOpen(true);
            }
          }}>{pickingComparison || comparing ? t.endCompare : t.compare}</button>
        <WorkspaceComparisonTrigger session={session} settings={settings} messages={messages}
          activePath={path} disabled={busy} />
        <div className="workspace-controls-spacer" />
        <div className="workspace-ide">
          <select aria-label={t.chooseIde} value={ide} onChange={(event) => setIde(event.target.value as ExternalIde)}>
            {Object.entries(IDES).map(([value, name]) => <option key={value} value={value}>{name}</option>)}
          </select>
          <button type="button" className="btn btn--ghost btn--xs" disabled={busy || active?.file.kind === "missing" || !!active?.unavailable}
            onClick={() => void session.openIde(path ?? "", ide)}>{t.openIde}</button>
        </div>
        <button type="button" className="btn btn--secondary btn--xs" aria-keyshortcuts="Meta+S Control+S"
          disabled={!dirty || busy} onClick={() => void session.save()}>{t.save}</button>
      </div>

      {fileForm && (
        <form className="workspace-file-form" onSubmit={async (event) => {
          event.preventDefault();
          const done = fileForm.rename
            ? await session.rename(fileForm.rename, fileForm.value)
            : await session.create(fileForm.value);
          if (done) {
            setFileForm(null);
            setComparison(null);
          }
        }}>
          <input aria-label={t.filePath} placeholder={t.filePath} className="mono" required value={fileForm.value}
            onChange={(event) => setFileForm({ ...fileForm, value: event.target.value })} />
          <button className="btn btn--primary btn--xs" disabled={busy} type="submit">{fileForm.rename ? t.rename : t.newFile}</button>
          <button className="btn btn--ghost btn--xs" type="button" disabled={busy} onClick={() => setFileForm(null)}>{t.cancel}</button>
        </form>
      )}

      {state.error && <p className="workspace-error" role="alert">{formatDomainError(state.error, settings.locale)}</p>}
      {active?.unavailable && <p className="workspace-error" role="alert">{formatDomainError(active.unavailable, settings.locale)}</p>}
      {conflicted?.conflict && (
        <div ref={conflictBanner} className="workspace-conflict" role="region" tabIndex={-1} aria-label={t.changedOnDisk}>
          <div role="status">
            <strong>{conflicted.conflict.kind === "missing" ? t.deletedOnDisk : t.changedOnDisk}</strong>
            <span className="mono">{conflicted.file.path}</span>
          </div>
          <button type="button" className="btn btn--secondary btn--xs" onClick={() => session.resolveConflict(conflicted.file.path, true)}>{t.loadDisk}</button>
          <button type="button" className="btn btn--ghost btn--xs" onClick={() => session.resolveConflict(conflicted.file.path, false)}>{t.keepDraft}</button>
        </div>
      )}

      <div className="workspace-content">
        {treeOpen && <WorkspaceTree session={session} version={state.treeVersion} activePath={path}
          onFile={(filePath) => void openFile(filePath)} messages={t} locale={settings.locale} />}
        <div className="workspace-code">
          {state.tabs.length > 0 && (
            <div className="workspace-tabs" role="tablist" aria-label={t.tabs}>
              {state.tabs.map((tab, index) => {
                const filePath = tab.file.path;
                const selected = filePath === path;
                return (
                  <div className="workspace-tab" key={filePath} role="presentation" data-active={selected}>
                    <button type="button" role="tab" aria-selected={selected} tabIndex={selected ? 0 : -1}
                      className="mono" title={filePath} onClick={() => selectTab(tab)}
                      onKeyDown={(event) => handleTabKey(event, index)}>
                      <span>{filePath.slice(filePath.lastIndexOf("/") + 1)}</span>
                      {tab.draft !== null && <span className="workspace-draft" role="img" aria-label={t.unsaved} title={t.unsaved} />}
                    </button>
                    <button className="btn btn--ghost btn--icon btn--xs" type="button" disabled={busy}
                      aria-label={interpolate(t.closeTab, { path: filePath })}
                      onClick={() => tab.draft !== null ? setClosingFile(filePath) : closeTab(filePath)}><CloseIcon size={12} /></button>
                  </div>
                );
              })}
            </div>
          )}

          {pickingComparison && <p className="workspace-comparison-message" role="status">{t.pickComparison}</p>}
          {comparing && leftTab && rightTab && (
            <div className="workspace-diff" role="region" aria-label={interpolate(t.comparisonName, { left: leftTab.file.path, right: rightTab.file.path })}>
              <div className="workspace-diff-head mono"><span>{leftTab.file.path}</span><span>{rightTab.file.path}</span></div>
              {left !== null && right !== null ? (
                useMonaco ? <Suspense fallback={<p className="workspace-empty">{t.loading}</p>}>
                  <MonacoDiff left={left} right={right} leftPath={leftTab.file.path} rightPath={rightTab.file.path} settings={settings} />
                </Suspense> : <div className="workspace-diff-head"><pre>{left}</pre><pre>{right}</pre></div>
              ) : <p className="workspace-empty">{t.comparisonUnavailable}</p>}
            </div>
          )}
          {text !== null && path !== null && (
            <div className="workspace-text-editor" hidden={comparing} role="tabpanel">
              {useMonaco ? (
                <Suspense fallback={<p className="workspace-empty" role="status">{t.loading}</p>}>
                  <MonacoText tabs={state.tabs} path={path} label={label} settings={settings} readOnly={switching} languageSession={session.language}
                    onEdit={(filePath, value) => session.change(filePath, value)} />
                </Suspense>
              ) : (
                <textarea aria-label={label} className="workspace-fallback mono" value={text} spellCheck={false} readOnly={switching}
                  onChange={(event) => session.change(path, event.target.value)} />
              )}
            </div>
          )}
          {!comparing && text === null && (
            <div className="workspace-empty">
              {active ? (
                <>
                  <strong>{active.file.kind === "tooLarge" ? t.tooLarge : active.file.kind === "binary" ? t.binary : t.missing}</strong>
                  <span className="mono">{active.file.path}</span>
                  {active.file.kind !== "missing" && <span className="mono">{interpolate(t.bytes, { size: active.file.sizeBytes })}</span>}
                </>
              ) : (
                <button type="button" className="btn btn--secondary btn--sm" onClick={() => setTreeOpen(true)}>{t.openFile}</button>
              )}
            </div>
          )}
          {active && <div className="workspace-status" role="status">
            <span className="mono workspace-status-path" title={active.file.path}>{active.file.path}</span>
            {editorText(active) !== null && <span className="workspace-status-message">{dirty ? t.unsaved : t.saved}</span>}
            {languageStatus && onOpenRuntimeSettings && (
              <button type="button" className="btn btn--ghost btn--xs workspace-language-status"
                onClick={(event) => onOpenRuntimeSettings(event.currentTarget)}>
                {interpolate(t.languageStatus[languageStatus.status], { language: languageStatus.language })}
              </button>
            )}
          </div>}
        </div>
      </div>

      {closingFile && (
        <WorkspaceDialog title={t.closeTitle} detail={<span className="mono">{closingFile}</span>} cancel={t.cancel} busy={busy}
          onCancel={() => setClosingFile(null)}>
          <button type="button" className="btn btn--secondary btn--sm" disabled={busy} onClick={() => closeTab(closingFile)}>{t.discard}</button>
          <button type="button" className="btn btn--primary btn--sm" disabled={busy} onClick={async () => {
            if (await session.save(closingFile)) closeTab(closingFile);
            else setClosingFile(null);
          }}>{t.saveAndClose}</button>
        </WorkspaceDialog>
      )}
    </div>
  );
}
