import { lazy, Suspense, useCallback, useEffect, useId, useMemo, useState } from "react";
import { CourseWorkbench } from "../components/CourseWorkbench";
import { Select } from "../components/Select";
import { expectResponse, type CoreClient } from "../core/client";
import type { CourseDraft, DraftPreview, GlobalSettings, WorkspaceFile } from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";
import { domainError, editorText, type EditorTab } from "../workspace/editorSession";
const MonacoText = lazy(() => import("../workspace/MonacoSurface").then((module) => ({ default: module.MonacoTextSurface })));

export interface DraftNavigation {
  hasDrafts: () => boolean;
  save: () => Promise<boolean>;
}

export function DraftWorkbench({
  client, draft, messages, settings, switching, focusMode, onToggleFocusMode,
  onGuard, onTrial, onCopyCommand,
}: {
  client: CoreClient;
  draft: CourseDraft;
  messages: Messages;
  settings: GlobalSettings;
  switching: boolean;
  focusMode: boolean;
  onToggleFocusMode: () => void;
  onGuard: (guard: DraftNavigation | null) => void;
  onTrial: () => Promise<void>;
  onCopyCommand: (text: string) => void;
}) {
  const t = messages.coursePack;
  const fileId = useId();
  const [path, setPath] = useState(draft.files.includes("README.md") ? "README.md" : draft.files[0] ?? "");
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [chapter, setChapter] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [conflict, setConflict] = useState<WorkspaceFile | null>(null);
  const selected = tabs.find(({ file }) => file.path === path);
  const text = selected ? editorText(selected) : null;
  const edits = useMemo(() => tabs.flatMap(({ file, draft }) => draft === null ? [] : [{ path: file.path, text: draft }]), [tabs]);
  const [preview, setPreview] = useState<{ value: DraftPreview; edits: typeof edits }>({ value: draft.preview, edits });
  const previewCurrent = preview.edits === edits;
  const course = preview.value.course;
  const disabled = busy || switching;

  useEffect(() => {
    if (!path || selected) return;
    let cancelled = false;
    void client.dispatch({ type: "readCourseDraftFile", payload: { draftId: draft.id, path } }).then((response) => {
      if (cancelled) return;
      const file = expectResponse(response, "workspaceFile").payload;
      if (file.path !== path) setPath(file.path);
      setTabs((current) => [...current.filter((tab) => tab.file.path !== file.path), { file, draft: null, conflict: null, unavailable: null }]);
    }).catch((error) => {
      if (!cancelled) setError(formatDomainError(domainError(error), settings.locale));
    });
    return () => { cancelled = true; };
  }, [client, draft.id, path, selected, settings.locale]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void client.dispatch({ type: "previewCourseDraft", payload: { draftId: draft.id, edits } }).then((response) => {
        if (!cancelled) setPreview({ value: expectResponse(response, "draftPreview").payload, edits });
      }).catch((error) => {
        if (!cancelled) setError(formatDomainError(domainError(error), settings.locale));
      });
    }, 180);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [client, draft.id, edits, settings.locale]);

  const save = useCallback(async () => {
    if (busy || conflict) return false;
    setBusy(true);
    setError(null);
    try {
      for (const tab of tabs) {
        const { file, draft: text } = tab;
        if (text === null) continue;
        const request = { draftId: draft.id, path: file.path, text, expectedRevision: file.revision };
        const response = await client.dispatch({ type: "writeCourseDraftFile", payload: request });
        if (response.type === "workspaceFileConflict") {
          setConflict(response.payload);
          setPath(file.path);
          return false;
        }
        const saved = expectResponse(response, "workspaceFile").payload;
        setTabs((current) => current.map((item) => item.file.path === file.path
          ? { ...item, file: saved, draft: item.draft === text ? null : item.draft } : item));
      }
      return true;
    } catch (error) {
      setError(formatDomainError(domainError(error), settings.locale));
      return false;
    } finally {
      setBusy(false);
    }
  }, [busy, client, conflict, draft.id, settings.locale, tabs]);

  useEffect(() => {
    onGuard({ hasDrafts: () => edits.length > 0, save });
    return () => onGuard(null);
  }, [edits.length, onGuard, save]);

  function edit(filePath: string, text: string) {
    setTabs((current) => current.map((tab) => tab.file.path === filePath
      ? { ...tab, draft: tab.file.kind === "text" && tab.file.text === text ? null : text }
      : tab));
  }

  function resolveConflict(keepDraft: boolean) {
    if (!conflict) return;
    setTabs((current) => current.map((tab) => tab.file.path === conflict.path
      ? { ...tab, file: conflict, draft: keepDraft ? tab.draft : null } : tab));
    setConflict(null);
  }

  const stage = (
    <div className="workspace-editor">
      <div className="workspace-location">
        <span className="badge badge--neutral">{t.draftTitle}</span>
        <span className="mono workspace-root" title={draft.root}>{draft.root}</span>
      </div>
      <div className="workspace-controls draft-controls">
        <label htmlFor={fileId}>{t.sourceFile}</label>
        <Select
          id={fileId}
          size="sm"
          mono
          className="draft-file-select"
          value={path}
          disabled={disabled}
          options={draft.files.map((file) => ({ value: file, label: file }))}
          onChange={(nextPath) => {
            setPath(nextPath);
            setError(null);
          }}
        />
        <button type="button" className="btn btn--secondary btn--sm"
          disabled={disabled || edits.length === 0 || conflict !== null} onClick={() => void save()}>
          {messages.workspace.save}
        </button>
        <button type="button" className="btn btn--primary btn--sm" title={t.trialHint}
          disabled={disabled || !previewCurrent || !course || preview.value.diagnostics.length > 0 || conflict !== null}
          onClick={() => void save().then((saved) => { if (saved) return onTrial(); })}>
          {t.trial}
        </button>
      </div>
      {error && <div role="alert" className="inline-alert inline-alert--danger">{error}</div>}
      {conflict && (
        <div role="alert" className="inline-alert inline-alert--danger">
          {t.draftConflict}
          <button type="button" className="btn btn--secondary btn--xs" onClick={() => resolveConflict(false)}>{messages.workspace.loadDisk}</button>
          <button type="button" className="btn btn--secondary btn--xs" onClick={() => resolveConflict(true)}>{messages.workspace.keepDraft}</button>
        </div>
      )}
      {preview.value.diagnostics.length > 0 && (
        <section className="inline-alert inline-alert--danger draft-diagnostics" aria-label={t.diagnostics} aria-live="polite">
          <strong>{t.diagnostics}</strong>
          <ul>
            {preview.value.diagnostics.map(({ path, error }) => {
              const { reasonCode, field, line, column } = error.details ?? {};
              return <li key={path}>
                <span className="mono">{path}{line && line !== "0" ? `:${line}:${column}` : ""}</span>
                {field && <> <code>{field}</code></>}
                <div>{t.diagnosticReasons[reasonCode] ?? formatDomainError(error, settings.locale)}</div>
              </li>;
            })}
          </ul>
        </section>
      )}
      <div className="workspace-text-editor" aria-busy={!previewCurrent}>
        {text !== null ? typeof ResizeObserver !== "undefined"
          ? <Suspense fallback={<p className="workspace-empty">{messages.workspace.loading}</p>}>
              <MonacoText tabs={tabs} path={path} label={t.sourceEditor} settings={settings} readOnly={disabled} onEdit={edit} />
            </Suspense>
          : <textarea className="workspace-fallback mono" aria-label={t.sourceEditor}
              aria-invalid={preview.value.diagnostics.some((item) => item.path === path)}
              readOnly={disabled} value={text} onChange={(event) => edit(path, event.target.value)} spellCheck={false} />
          : <p className="workspace-empty">{selected ? messages.workspace.openFile : messages.workspace.loading}</p>}
      </div>
    </div>
  );

  return course ? (
    <CourseWorkbench
      course={course} session={null} settings={settings} switching={switching}
      recentWorkspacePath={null} activeChapterId={chapter} focusMode={focusMode}
      pendingReload={null} reloading={false} messages={messages} stage={stage}
      onChooseWorkspace={() => {}} onResumeWorkspace={() => {}} onRequestTrust={() => {}} onRevokeTrust={() => {}}
      onSelectChapter={setChapter} onToggleFocusMode={onToggleFocusMode} onReloadCourse={() => {}}
      onDismissReload={() => {}} onCopyCommand={onCopyCommand}
    />
  ) : (
    <section className="wb" aria-label={t.draftTitle}>
      <aside className="pane-left"><div className="workspace-stage">{t.previewUnavailable}</div></aside>
      <section className="pane-center">{stage}</section>
    </section>
  );
}
