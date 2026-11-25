import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  courseSourceLocation,
  type Chapter,
  type GlobalSettings,
  type CommandBlock,
  type CourseSourceChangedEvent,
  type LoadedCourse,
} from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { ChapterOutlineDrawer } from "./ChapterOutlineDrawer";
import { CommandBlockCard } from "./CommandBlockCard";
import {
  BookOpenIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  FileCodeIcon,
  FolderIcon,
  MenuIcon,
  PanelLeftOpenIcon,
  RefreshIcon,
} from "./Icons";

import { WorkspaceEditor } from "../workspace/WorkspaceEditor";
import type { WorkspaceEditorSession } from "../workspace/editorSession";

const DEFAULT_MANUAL_WIDTH_PCT = 38;
const MIN_MANUAL_WIDTH_PX = 320;
const MAX_MANUAL_WIDTH_PCT = 54;

export interface CourseWorkbenchProps {
  course: LoadedCourse;
  session: WorkspaceEditorSession | null;
  settings: GlobalSettings;
  switching: boolean;
  recentWorkspacePath: string | null;
  onChooseWorkspace: () => void;
  onResumeWorkspace: () => void;
  onRequestTrust: () => void;
  onRevokeTrust: () => void;
  activeChapterId: string;
  focusMode: boolean;
  pendingReload: CourseSourceChangedEvent | null;
  reloading: boolean;
  messages: Messages;
  onSelectChapter: (chapterId: string) => void;
  onToggleFocusMode: () => void;
  onReloadCourse: () => void;
  onDismissReload: () => void;
  onCopyCommand: (commandText: string) => void;
  stage?: ReactNode;
  onReturnToDraft?: () => void;
}

function getFileBasename(filePath: string): string {
  const parts = filePath.split(/[/\\]/);
  return parts[parts.length - 1] || filePath;
}

export function CourseWorkbench({
  course,
  session,
  settings,
  switching,
  recentWorkspacePath,
  onChooseWorkspace,
  onResumeWorkspace,
  onRequestTrust,
  onRevokeTrust,
  activeChapterId,
  focusMode,
  pendingReload,
  reloading,
  messages,
  onSelectChapter,
  onToggleFocusMode,
  onReloadCourse,
  onDismissReload,
  onCopyCommand,
  stage,
  onReturnToDraft,
}: CourseWorkbenchProps) {
  const [outlineOpen, setOutlineOpen] = useState(false);
  const [manualWidthPx, setManualWidthPx] = useState<number | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  const wbRef = useRef<HTMLDivElement | null>(null);
  const manualScrollRef = useRef<HTMLDivElement | null>(null);

  const activeIndex = useMemo(() => {
    const idx = course.chapters.findIndex(
      (ch) => ch.chapterId === activeChapterId
    );
    return idx >= 0 ? idx : 0;
  }, [course.chapters, activeChapterId]);

  const activeChapter: Chapter | undefined = course.chapters[activeIndex];
  const prevChapter = activeIndex > 0 ? course.chapters[activeIndex - 1] : null;
  const nextChapter =
    activeIndex + 1 < course.chapters.length
      ? course.chapters[activeIndex + 1]
      : null;

  // Scroll manual back to top when switching chapters
  useEffect(() => {
    if (manualScrollRef.current) {
      manualScrollRef.current.scrollTop = 0;
    }
  }, [activeChapter?.chapterId]);

  const isOutlineVisible = outlineOpen && !focusMode;

  const clampManualWidth = useCallback((clientX: number): number | null => {
    const container = wbRef.current;
    if (!container) return null;
    const rect = container.getBoundingClientRect();
    const containerWidth = rect.width || window.innerWidth || 1280;
    const maxPx = Math.max(
      MIN_MANUAL_WIDTH_PX,
      Math.round((containerWidth * MAX_MANUAL_WIDTH_PCT) / 100)
    );
    const rawPx = Math.round(clientX - rect.left);
    return Math.min(maxPx, Math.max(MIN_MANUAL_WIDTH_PX, rawPx));
  }, []);

  const handleSplitterMouseDown = (
    event: React.MouseEvent<HTMLDivElement>
  ) => {
    if (focusMode) return;
    event.preventDefault();
    setIsDragging(true);

    const handleMove = (moveEvent: MouseEvent) => {
      const clamped = clampManualWidth(moveEvent.clientX);
      if (clamped !== null) {
        setManualWidthPx(clamped);
      }
    };

    const handleUp = () => {
      setIsDragging(false);
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
  };

  const handleSplitterDoubleClick = () => {
    setManualWidthPx(null);
  };

  const handleSplitterKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (focusMode) return;
    const containerWidth = wbRef.current?.getBoundingClientRect().width || 1280;
    const currentPx =
      manualWidthPx ?? Math.round((containerWidth * DEFAULT_MANUAL_WIDTH_PCT) / 100);
    const maxPx = Math.max(
      MIN_MANUAL_WIDTH_PX,
      Math.round((containerWidth * MAX_MANUAL_WIDTH_PCT) / 100)
    );
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      setManualWidthPx(Math.max(MIN_MANUAL_WIDTH_PX, currentPx - 24));
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      setManualWidthPx(Math.min(maxPx, currentPx + 24));
    } else if (event.key === "Home" || event.key === "Enter") {
      event.preventDefault();
      setManualWidthPx(null);
    }
  };

  const t = messages.workbench;

  const exerciseOrdinal = useMemo(() => {
    if (!activeChapter || activeChapter.kind !== "exercise") return null;
    let count = 0;
    for (const ch of course.chapters) {
      if (ch.kind === "exercise") {
        count += 1;
      }
      if (ch.chapterId === activeChapter.chapterId) {
        return count;
      }
    }
    return null;
  }, [course.chapters, activeChapter]);

  const commandById = useMemo(() => {
    const map = new Map<string, CommandBlock>();
    if (activeChapter) {
      for (const cmd of activeChapter.commands) {
        map.set(cmd.id, cmd);
      }
    }
    return map;
  }, [activeChapter]);

  const paneLeftStyle =
    !focusMode && manualWidthPx !== null
      ? { width: `${manualWidthPx}px` }
      : undefined;

  const sourceBadge =
    course.source.kind === "builtin"
      ? t.builtinBadge
      : course.source.kind === "gitPack"
        ? messages.coursePack.gitBadge
        : course.source.kind === "localPack"
        ? t.localPackBadge
        : t.localMarkdownBadge;

  const sourceLocation = courseSourceLocation(course.source);
  const w = messages.workspace;

  const threeStep = activeChapter?.threeStep ?? null;
  const isThreeStepMode =
    activeChapter?.presentation === "threeStep" && threeStep !== null;
  const chapterFile = threeStep?.step2EntryFile;
  const activeReflectionsHtml = isThreeStepMode
    ? threeStep.reflectionsHtml
    : (activeChapter?.reflectionsHtml ?? null);

  const visibleStepNumbers = useMemo(() => {
    if (!threeStep) {
      return { step1: null, step2: null, step3: null };
    }
    const hasStep1 = threeStep.step1CommandIds.length > 0;
    const hasStep2 =
      Boolean(threeStep.step2Html) || Boolean(threeStep.step2EntryFile);
    const hasStep3 =
      threeStep.step3Checks.length > 0 ||
      Boolean(threeStep.step3IntroHtml) ||
      Boolean(threeStep.requiresDaemon);

    const step1 = hasStep1 ? 1 : null;
    const step2 = hasStep2 ? (step1 ?? 0) + 1 : null;
    const step3 = hasStep3 ? (step2 ?? step1 ?? 0) + 1 : null;
    return { step1, step2, step3 };
  }, [threeStep]);

  const secondaryCommands = useMemo(() => {
    if (!threeStep) return [];
    return threeStep.secondaryCommandIds
      .map((id) => commandById.get(id))
      .filter((cmd): cmd is CommandBlock => cmd !== undefined);
  }, [threeStep, commandById]);

  const reloadFileLabel = pendingReload?.changedPaths[0]
    ? getFileBasename(pendingReload.changedPaths[0])
    : course.title;

  return (
    <section
      ref={wbRef}
      className={`wb${focusMode ? " is-focus" : ""}${isDragging ? " is-resizing" : ""}`}
      aria-label={t.ariaLabel}
      data-testid="course-workbench"
    >
      {/* Left Pane: Guided 3-Step / Linear Manual */}
      <aside
        className="pane-left"
        style={paneLeftStyle}
        aria-label={t.manualAriaLabel}
        data-testid="manual-pane"
      >
        <div className="focus-rail">
          <button
            type="button"
            className="btn btn--ghost btn--icon btn--sm"
            aria-label={t.focusRailExpand}
            title={t.focusRailExpand}
            onClick={onToggleFocusMode}
          >
            <PanelLeftOpenIcon size={16} />
          </button>
          <span className="focus-rail-text">
            {activeChapter?.title ?? t.focusRailLabel}
          </span>
        </div>

        <div className="manual-head manual-head-full">
          <button
            type="button"
            className="btn btn--ghost btn--icon btn--xs"
            aria-label={t.outlineToggle}
            aria-expanded={isOutlineVisible}
            title={t.outlineToggle}
            onClick={() => setOutlineOpen((open) => !open)}
          >
            <MenuIcon size={15} />
          </button>

          <span
            className={`manual-head-title${
              activeChapter?.kind === "exercise" && exerciseOrdinal !== null
                ? " mono"
                : ""
            }`}
            data-testid="manual-chapter-pos"
          >
            {activeChapter?.kind === "exercise" && exerciseOrdinal !== null
              ? interpolate(t.exerciseCounter, {
                  current: exerciseOrdinal,
                  total: course.totalExercises,
                })
              : `${t.guideHeaderBadge} · ${activeChapter?.title ?? ""}`}
          </span>

          <div className="manual-nav-btns">
            <button
              type="button"
              className="btn btn--ghost btn--icon btn--xs"
              aria-label={t.prevChapter}
              title={t.prevChapter}
              disabled={!prevChapter}
              onClick={() => {
                if (prevChapter) onSelectChapter(prevChapter.chapterId);
              }}
            >
              <ChevronLeftIcon size={14} />
            </button>
            <button
              type="button"
              className="btn btn--ghost btn--icon btn--xs"
              aria-label={t.nextChapter}
              title={t.nextChapter}
              disabled={!nextChapter}
              onClick={() => {
                if (nextChapter) onSelectChapter(nextChapter.chapterId);
              }}
            >
              <ChevronRightIcon size={14} />
            </button>
          </div>
        </div>

        <ChapterOutlineDrawer
          open={isOutlineVisible}
          chapters={course.chapters}
          totalExercises={course.totalExercises}
          activeChapterId={activeChapter?.chapterId ?? ""}
          messages={messages}
          onSelectChapter={onSelectChapter}
          onClose={() => setOutlineOpen(false)}
        />

        {course.source.kind === "gitPack" && (
          <div className="git-course-actions">
            <button type="button" className="btn btn--secondary btn--xs"
              disabled={reloading || switching} onClick={onReloadCourse}>
              <RefreshIcon size={12} />
              {reloading ? messages.coursePack.gitLoading : messages.coursePack.gitRefresh}
            </button>
          </div>
        )}

        {pendingReload ? (
          <div
            className="reload-banner"
            role="status"
            data-testid="hot-reload-banner"
          >
            <div className="reload-copy">
              <strong>{t.reloadTitle}</strong>
              <span>
                {interpolate(t.reloadDetail, { file: reloadFileLabel })}
              </span>
            </div>
            <div className="reload-actions">
              <button
                type="button"
                className="btn btn--primary btn--xs btn--pill"
                disabled={reloading}
                onClick={onReloadCourse}
              >
                <RefreshIcon size={12} />
                <span>{t.reloadAction}</span>
              </button>
              <button
                type="button"
                className="btn btn--ghost btn--xs"
                onClick={onDismissReload}
              >
                {t.reloadDismiss}
              </button>
            </div>
          </div>
        ) : null}

        <div ref={manualScrollRef} className="manual-scroll">
          {activeChapter ? (
            <div className="manual-inner">
              {activeIndex === 0 && course.introHtml ? (
                <div
                  className="course-intro md-prose"
                  data-testid="course-intro"
                  dangerouslySetInnerHTML={{ __html: course.introHtml }}
                />
              ) : null}

              <header className="manual-hero">
                <div className="manual-hero-meta">
                  <span
                    className={`badge ${
                      activeChapter.kind === "guide"
                        ? "badge--neutral"
                        : "badge--accent"
                    }`}
                    data-testid="chapter-kind-badge"
                  >
                    {activeChapter.kind === "guide"
                      ? t.chapterKindGuide
                      : t.chapterKindExercise}
                  </span>
                  <span className="mono chapter-id-chip">
                    #{activeChapter.chapterId}
                  </span>
                </div>
                <h2>{activeChapter.title}</h2>
                {isThreeStepMode && activeChapter.summaryHtml ? (
                  <div
                    className="md-prose md-prose--summary"
                    dangerouslySetInnerHTML={{
                      __html: activeChapter.summaryHtml,
                    }}
                  />
                ) : null}
              </header>

              {isThreeStepMode ? (
                <div
                  className="three-step-flow"
                  data-testid="three-step-flow"
                >
                  {visibleStepNumbers.step1 !== null ? (
                    <section
                      className="step-section"
                      data-testid="step-1-section"
                    >
                      <div className="step-head">
                        <span className="step-num">
                          {visibleStepNumbers.step1}
                        </span>
                        <h3>{t.step1Title}</h3>
                      </div>
                      {threeStep.step1IntroHtml ? (
                        <div
                          className="md-prose"
                          dangerouslySetInnerHTML={{
                            __html: threeStep.step1IntroHtml,
                          }}
                        />
                      ) : null}
                      {threeStep.step1CommandIds.map((cmdId) => {
                        const cmd = commandById.get(cmdId);
                        if (!cmd) return null;
                        return (
                          <CommandBlockCard
                            key={cmd.id}
                            command={cmd}
                            messages={messages}
                            onCopy={onCopyCommand}
                          />
                        );
                      })}
                    </section>
                  ) : null}

                  {visibleStepNumbers.step2 !== null ? (
                    <section
                      className="step-section"
                      data-testid="step-2-section"
                    >
                      <div className="step-head">
                        <span className="step-num">
                          {visibleStepNumbers.step2}
                        </span>
                        <h3>{t.step2Title}</h3>
                        {chapterFile ? (
                          course.trusted && session ? (
                            <button type="button" className="file-pill mono workspace-chapter-file"
                              aria-label={interpolate(w.openChapterFile, { path: chapterFile })}
                              disabled={switching}
                              onClick={() => void session.open(chapterFile)}>
                              <FileCodeIcon size={12} />
                              <span>{chapterFile}</span>
                            </button>
                          ) : <span className="file-pill mono">{chapterFile}</span>
                        ) : null}
                      </div>
                      {threeStep.step2Html ? (
                        <div
                          className="md-prose"
                          dangerouslySetInnerHTML={{
                            __html: threeStep.step2Html,
                          }}
                        />
                      ) : null}
                    </section>
                  ) : null}

                  {visibleStepNumbers.step3 !== null ? (
                    <section
                      className="step-section"
                      data-testid="step-3-section"
                    >
                      <div className="step-head">
                        <span className="step-num">
                          {visibleStepNumbers.step3}
                        </span>
                        <h3>{t.step3Title}</h3>
                      </div>

                      <div className="verify-card">
                        {threeStep.requiresDaemon ? (
                          <div className="daemon-req-note">
                            <span className="dot dot--ok" aria-hidden="true" />
                            <span>
                              {interpolate(t.requiresDaemonLabel, {
                                label: threeStep.requiresDaemon.label,
                                port: threeStep.requiresDaemon.port,
                              })}
                            </span>
                          </div>
                        ) : null}

                        {threeStep.step3IntroHtml ? (
                          <div
                            className="md-prose"
                            dangerouslySetInnerHTML={{
                              __html: threeStep.step3IntroHtml,
                            }}
                          />
                        ) : null}

                        {threeStep.step3Checks.map((check) => (
                          <div key={check.id} className="v-item">
                            <div className="v-row">
                              <span className="v-check" aria-hidden="true" />
                              <span className="v-text">{check.title}</span>
                              <span className="badge badge--neutral v-kind">
                                {t.checkKinds[check.kind]}
                              </span>
                            </div>
                          </div>
                        ))}
                      </div>
                    </section>
                  ) : null}

                  {secondaryCommands.length > 0 ? (
                    <details
                      className="disclosure"
                      data-testid="secondary-commands-details"
                    >
                      <summary>
                        <span>
                          {interpolate(t.secondaryCommandsTitle, {
                            count: secondaryCommands.length,
                          })}
                        </span>
                      </summary>
                      <div className="disclosure-body">
                        {secondaryCommands.map((cmd) => (
                          <CommandBlockCard
                            key={cmd.id}
                            command={cmd}
                            messages={messages}
                            onCopy={onCopyCommand}
                          />
                        ))}
                      </div>
                    </details>
                  ) : null}
                </div>
              ) : (
                <div
                  className="linear-flow"
                  data-testid="linear-fallback-flow"
                >
                  {!course.isBuiltin && activeChapter.kind === "guide" ? (
                    <div className="fallback-note">
                      <BookOpenIcon size={13} />
                      <span>{t.linearFallbackNotice}</span>
                    </div>
                  ) : null}

                  {activeChapter.blocks.map((block, blockIdx) => {
                    if (block.kind === "prose") {
                      return (
                        <div
                          key={`prose-${blockIdx}`}
                          className="md-prose"
                          dangerouslySetInnerHTML={{ __html: block.html }}
                        />
                      );
                    }
                    const cmd = commandById.get(block.commandId);
                    if (!cmd) return null;
                    return (
                      <CommandBlockCard
                        key={cmd.id}
                        command={cmd}
                        messages={messages}
                        onCopy={onCopyCommand}
                      />
                    );
                  })}
                </div>
              )}

              {activeReflectionsHtml ? (
                <details
                  className="disclosure"
                  data-testid="reflections-details"
                >
                  <summary>
                    <span>{t.reflectionsTitle}</span>
                  </summary>
                  <div className="disclosure-body">
                    <div
                      className="reflect-box md-prose"
                      dangerouslySetInnerHTML={{
                        __html: activeReflectionsHtml,
                      }}
                    />
                  </div>
                </details>
              ) : null}

              <footer className="manual-foot-nav">
                <button
                  type="button"
                  className="btn btn--secondary btn--sm manual-foot-btn"
                  disabled={!prevChapter}
                  onClick={() => {
                    if (prevChapter) onSelectChapter(prevChapter.chapterId);
                  }}
                >
                  <ChevronLeftIcon size={13} />
                  <span>{prevChapter ? prevChapter.title : t.prevChapter}</span>
                </button>
                <button
                  type="button"
                  className="btn btn--secondary btn--sm manual-foot-btn"
                  disabled={!nextChapter}
                  onClick={() => {
                    if (nextChapter) onSelectChapter(nextChapter.chapterId);
                  }}
                >
                  <span>{nextChapter ? nextChapter.title : t.nextChapter}</span>
                  <ChevronRightIcon size={13} />
                </button>
              </footer>
            </div>
          ) : null}
        </div>
      </aside>

      {/* Draggable Vertical Splitter (double-click resets width) */}
      <div
        className={`split-x${isDragging ? " is-drag" : ""}`}
        role="separator"
        aria-orientation="vertical"
        aria-label={t.splitterAriaLabel}
        tabIndex={focusMode ? -1 : 0}
        title={t.splitterTitle}
        data-testid="workbench-splitter"
        onMouseDown={handleSplitterMouseDown}
        onDoubleClick={handleSplitterDoubleClick}
        onKeyDown={handleSplitterKeyDown}
      />

      <section className="pane-center" aria-label={t.stageAriaLabel}>
        {stage ?? (session && course.trusted ? (
          <WorkspaceEditor
            key={session.workspace.id}
            session={session}
            settings={settings}
            messages={messages}
            switching={switching}
            onChangeWorkspace={onReturnToDraft ?? onChooseWorkspace}
            workspaceActionLabel={onReturnToDraft ? messages.coursePack.returnDraft : undefined}
            workspaceLabel={onReturnToDraft ? messages.coursePack.trialTitle : undefined}
            onRevokeTrust={course.isBuiltin || onReturnToDraft ? undefined : onRevokeTrust}
          />
        ) : (
          <>
            <div className="stage-bar">
              <div className="stage-bar-left">
                <span className="badge badge--neutral">{sourceBadge}</span>
                {!course.trusted && <span className="badge badge--warning">{t.readOnlyPreviewBadge}</span>}
              </div>
              {course.trusted && !course.isBuiltin && (
                <button type="button" className="btn btn--ghost btn--xs" disabled={switching} onClick={onRevokeTrust}>{w.revokeTrust}</button>
              )}
            </div>
            <div className="workspace-stage">
              <FolderIcon size={22} />
              <h3>{course.trusted ? w.choose : course.title}</h3>
              <p>{course.trusted ? w.chooseHint : w.previewHint}</p>
              {!course.trusted && <span className="mono workspace-source" title={sourceLocation}>{sourceLocation}</span>}
              <div className="workspace-stage-actions">
                {course.trusted ? (
                  <>
                    {recentWorkspacePath && <button type="button" className="btn btn--primary btn--sm"
                      disabled={switching} onClick={onResumeWorkspace}>{w.resume}</button>}
                    <button type="button" className={recentWorkspacePath ? "btn btn--secondary btn--sm" : "btn btn--primary btn--sm"}
                      disabled={switching} onClick={onChooseWorkspace}>{w.choose}</button>
                  </>
                ) : <button type="button" className="btn btn--primary btn--sm" disabled={switching} onClick={onRequestTrust}>{w.trust}</button>}
              </div>
            </div>
          </>
        ))}
      </section>
    </section>
  );
}
