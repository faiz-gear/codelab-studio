import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  courseSourceLocation,
  type Chapter,
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
  MenuIcon,
  PanelLeftOpenIcon,
  RefreshIcon,
} from "./Icons";

const DEFAULT_MANUAL_WIDTH_PCT = 38;
const MIN_MANUAL_WIDTH_PX = 320;
const MAX_MANUAL_WIDTH_PCT = 54;

export interface CourseWorkbenchProps {
  course: LoadedCourse;
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
}

function getFileBasename(filePath: string): string {
  const parts = filePath.split(/[/\\]/);
  return parts[parts.length - 1] || filePath;
}

export function CourseWorkbench({
  course,
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
      : course.source.kind === "localPack"
        ? t.localPackBadge
        : t.localMarkdownBadge;

  const sourceLocation = courseSourceLocation(course.source);

  const threeStep = activeChapter?.threeStep ?? null;
  const isThreeStepMode =
    activeChapter?.presentation === "threeStep" && threeStep !== null;
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
                        {threeStep.step2EntryFile ? (
                          <span className="file-pill mono">
                            <FileCodeIcon size={12} />
                            <span>{threeStep.step2EntryFile}</span>
                          </span>
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

      {/* Right Pane: Calm Read-Only Workbench Stage */}
      <section className="pane-center" aria-label={t.stageAriaLabel}>
        <div className="stage-bar">
          <div className="stage-bar-left">
            <span className="badge badge--neutral">{sourceBadge}</span>
            {!course.isBuiltin ? (
              <span className="badge badge--warning">
                {t.readOnlyPreviewBadge}
              </span>
            ) : null}
            {activeChapter?.entryFile ? (
              <span className="file-pill mono">
                <FileCodeIcon size={12} />
                <span>{activeChapter.entryFile}</span>
              </span>
            ) : null}
          </div>
          {activeChapter ? (
            <span className="mono muted stage-chapter-id">
              {activeChapter.chapterId}
            </span>
          ) : null}
        </div>

        <div className="stage-body">
          <div className="stage-inner">
            <header className="stage-head">
              <h3>{activeChapter?.title ?? course.title}</h3>
              <p className="muted">
                {activeChapter?.kind === "guide"
                  ? t.stageGuideHint
                  : t.stageExerciseHint}
              </p>
            </header>

            <div className="stage-grid">
              <div className="stage-card">
                <div className="stage-card-head">
                  <span>{t.stageMetadataHeading}</span>
                </div>
                <dl className="stage-dl">
                  <div>
                    <dt>{t.stageChapterIdLabel}</dt>
                    <dd className="mono">{activeChapter?.chapterId ?? "—"}</dd>
                  </div>
                  <div>
                    <dt>{t.stageSourceLabel}</dt>
                    <dd className="mono stage-path" title={sourceLocation}>
                      {sourceLocation}
                    </dd>
                  </div>
                  {activeChapter?.entryFile ? (
                    <div>
                      <dt>{t.targetFileLabel}</dt>
                      <dd className="mono">{activeChapter.entryFile}</dd>
                    </div>
                  ) : null}
                </dl>
              </div>

              <div className="stage-card">
                <div className="stage-card-head">
                  <span>{t.stageCommandsHeading}</span>
                  <span className="mono muted">
                    {activeChapter?.commands.length ?? 0}
                  </span>
                </div>
                {activeChapter && activeChapter.commands.length > 0 ? (
                  <ul className="stage-list">
                    {activeChapter.commands.map((cmd) => (
                      <li key={cmd.id} className="stage-list-row">
                        <span className="mono stage-cmd-preview">
                          {cmd.command.split("\n")[0]}
                        </span>
                        <span className="mono muted stage-cwd-meta">
                          {cmd.cwd ?? "."}
                        </span>
                        <span
                          className={`badge ${
                            cmd.mode === "daemon"
                              ? "badge--info"
                              : "badge--neutral"
                          }`}
                        >
                          {cmd.mode === "daemon"
                            ? t.commandModeDaemon
                            : t.commandModeOneshot}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="muted stage-empty">{t.stageNoCommands}</p>
                )}
              </div>

              <div className="stage-card">
                <div className="stage-card-head">
                  <span>{t.stageChecksHeading}</span>
                  <span className="mono muted">
                    {threeStep?.step3Checks.length ?? 0}
                  </span>
                </div>
                {threeStep && threeStep.step3Checks.length > 0 ? (
                  <ul className="stage-list">
                    {threeStep.step3Checks.map((check) => (
                      <li key={check.id} className="stage-list-row">
                        <span className="stage-check-title">{check.title}</span>
                        <span className="badge badge--neutral">
                          {t.checkKinds[check.kind]}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="muted stage-empty">{t.stageNoChecks}</p>
                )}
              </div>
            </div>

            <p className="stage-foot-note muted">{t.stageNextMilestoneNote}</p>
          </div>
        </div>
      </section>
    </section>
  );
}
