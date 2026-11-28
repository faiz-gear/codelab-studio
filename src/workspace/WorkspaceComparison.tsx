import { lazy, Suspense, useEffect, useId, useState } from "react";
import { Select } from "../components/Select";
import type { GlobalSettings } from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";
import { domainError, type ComparisonReview, type WorkspaceEditorSession } from "./editorSession";
import { WorkspaceDialog } from "./WorkspaceDialog";

const MonacoDiff = lazy(() => import("./MonacoSurface").then((module) => ({ default: module.MonacoDiffSurface })));

type ComparisonProps = {
  session: WorkspaceEditorSession;
  settings: GlobalSettings;
  messages: Messages;
  chapterId?: string;
  activePath?: string | null;
};

export function WorkspaceComparisonTrigger({ disabled, ...props }: ComparisonProps & { disabled: boolean }) {
  const [open, setOpen] = useState(false);
  const { comparison } = props.messages;
  return (
    <>
      <button type="button" className="btn btn--secondary btn--xs"
        disabled={disabled} onClick={() => setOpen(true)}>
        {props.chapterId === undefined ? comparison.starterTrigger : comparison.referenceTrigger}
      </button>
      {open && <ComparisonDialog {...props} close={() => setOpen(false)} />}
    </>
  );
}

function ComparisonDialog({ session, settings, messages, chapterId, activePath, close }: ComparisonProps & { close: () => void }) {
  const [reviews, setReviews] = useState<ComparisonReview[]>([]);
  const [paths, setPaths] = useState<string[] | null>(null);
  const [path, setPath] = useState("");
  const [starterReview, setStarterReview] = useState<ComparisonReview>();
  const [error, setError] = useState<string>();
  const [readyPath, setReadyPath] = useState("");
  const [stale, setStale] = useState(false);
  const [applying, setApplying] = useState(false);
  const pickerId = useId();
  const starter = chapterId === undefined;
  const { comparison: t, workspace: w } = messages;

  useEffect(() => {
    let active = true;
    const failed = (error: unknown) => {
      if (active) setError(formatDomainError(domainError(error), settings.locale));
    };
    if (chapterId === undefined) {
      void session.listStarterFiles().then((files) => {
        if (!active) return;
        setPaths(files);
        if (activePath && files.includes(activePath)) setPath(activePath);
      }, failed);
    } else {
      void session.previewChapterSolution(chapterId).then((files) => {
        if (!active) return;
        setReviews(files);
        const paths = files.map(({ comparison }) => comparison.current.path);
        setPaths(paths);
        setPath(paths[0] ?? "");
      }, failed);
    }
    return () => { active = false; };
  }, [activePath, chapterId, session, settings.locale]);

  useEffect(() => {
    if (!starter || !path) return;
    let active = true;
    void session.previewStarterFile(path).then((file) => {
      if (active) setStarterReview(file);
    }, (error: unknown) => {
      if (active) setError(formatDomainError(domainError(error), settings.locale));
    });
    return () => { active = false; };
  }, [path, session, settings.locale, starter]);

  const review = starter ? starterReview : reviews.find(({ comparison }) => comparison.current.path === path);
  const compared = review?.comparison.current.path === path ? review : undefined;
  const file = compared?.comparison.current;
  const original = compared?.draft ?? (file?.kind === "text" ? file.text : "");
  const useMonaco = typeof ResizeObserver !== "undefined";
  const canApply = Boolean(file && !applying && !stale && !error && (!useMonaco || readyPath === path));
  const title = starter ? t.starterTitle : t.referenceTitle;

  async function apply() {
    if (!compared || !canApply) return;
    setApplying(true);
    const outcome = await session.applyComparison(compared);
    setApplying(false);
    if (outcome === "stale") {
      setStale(true);
    } else if (!starter && outcome === "applied" && reviews.length > 1) {
      const remaining = reviews.filter(({ comparison }) => comparison.current.path !== path);
      const remainingPaths = remaining.map(({ comparison }) => comparison.current.path);
      setReadyPath("");
      setReviews(remaining);
      setPaths(remainingPaths);
      setPath(remainingPaths[0]);
    } else {
      close();
    }
  }

  return (
    <WorkspaceDialog title={title} detail={t.replaceFile} cancel={w.cancel} onCancel={close} busy={applying}
      className="workspace-snapshot-dialog"
      body={
        <div className="workspace-snapshot-body">
          {paths && paths.length > 0 && (
            <div className="workspace-snapshot-picker">
              <label htmlFor={pickerId}>{t.chooseFile}</label>
              <Select
                id={pickerId}
                size="sm"
                mono
                className="workspace-snapshot-select"
                value={path}
                disabled={applying}
                placeholder={starter ? t.selectFile : undefined}
                options={paths.map((item) => ({ value: item, label: item }))}
                onChange={(nextPath) => {
                  setReadyPath("");
                  setPath(nextPath);
                  setStarterReview(undefined);
                  setError(undefined);
                  setStale(false);
                }}
              />
            </div>
          )}
          {error && <p className="workspace-error" role="alert">{error}</p>}
          {stale && <p className="workspace-error" role="alert">{t.stale}</p>}
          {!error && (!paths || (path && !file)) && <p role="status">{w.loading}</p>}
          {paths?.length === 0 && <p>{starter ? t.starterEmpty : t.referenceEmpty}</p>}
          {compared && file && (
            <div className="workspace-diff workspace-snapshot-diff" role="region" aria-label={`${t.currentFile} · ${starter ? t.starter : t.referenceTitle}`}>
              <div className="workspace-diff-head">
                <div>
                  <strong>{t.currentFile}</strong>
                  {compared.draft !== null ? <span className="workspace-snapshot-note">{t.draft}</span> : file.kind === "missing" && <span className="workspace-snapshot-note">{w.missing}</span>}
                </div>
                <strong>{starter ? t.starter : t.referenceTitle}</strong>
              </div>
              {useMonaco ? (
                <Suspense fallback={<p role="status">{w.loading}</p>}>
                  <MonacoDiff left={original} right={compared.comparison.proposed} leftPath={path} rightPath={path}
                    leftLabel={t.currentFile} rightLabel={starter ? t.starter : t.referenceTitle}
                    settings={settings} showWhitespace onReady={setReadyPath} />
                </Suspense>
              ) : (
                <div className="workspace-diff-head mono"><pre>{original}</pre><pre>{compared.comparison.proposed}</pre></div>
              )}
            </div>
          )}
        </div>
      }>
      <button type="button" className="btn btn--primary btn--sm"
        disabled={!canApply} onClick={() => void apply()}>
        {starter ? t.starterApply : t.referenceApply}
      </button>
    </WorkspaceDialog>
  );
}
