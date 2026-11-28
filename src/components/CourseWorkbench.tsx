import { useVerification } from "../verification/useVerification";
import { CheckResults } from "../verification/CheckResults";
import { WorkspaceDialog } from "../workspace/WorkspaceDialog";
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore, type CSSProperties, type ReactNode } from "react";
import {
  courseSourceLocation,
  type Chapter,
  type ContextQuote,
  type GlobalSettings,
  type CommandBlock,
  type CourseSourceChangedEvent,
  type LoadedCourse,
  type LocalMetricEvent,
  type MetricSelectionAction,
  type TutorSessionSnapshot,
  type UpdateSettingsPatch,
} from "../core/contracts";
import { interpolate, type Messages } from "../i18n";
import { SelectionAskPill } from "../tutor/SelectionAskPill";
import { TutorDrawer } from "../tutor/TutorDrawer";
import {
  extractDomSelectionWithin,
  formatCheckFailureQuote,
  htmlToPlainText,
  MIN_SELECTION_CHARS,
  quickPromptForSelectionAction,
  type ActiveSelection,
  type EditorCursorContext,
  type SelectionQuickAction,
  type TutorAskRequest,
} from "../tutor/contextRequest";
import { ChapterOutlineDrawer } from "./ChapterOutlineDrawer";
import type { CoreClient } from "../core/client";
import { CommandWorkbench, emptyWorkbench } from "../execution/CommandWorkbench";
import { CommandConfirmationDialog, CommandCwdDialog } from "../execution/ExecutionDialogs";
import { TerminalPanel } from "../execution/TerminalPanel";
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
  SparklesIcon,
} from "./Icons";

import { WorkspaceComparisonTrigger } from "../workspace/WorkspaceComparison";
import { WorkspaceEditor } from "../workspace/WorkspaceEditor";
import type { WorkspaceEditorSession } from "../workspace/editorSession";
import {
  clamp,
  computeBottomBounds,
  computeDockedTutorBounds,
  computeManualBounds,
  DEFAULT_TERMINAL_HEIGHT_PX,
  DEFAULT_VERIFICATION_HEIGHT_PX,
  resolveHorizontalWorkbenchWidths,
} from "./workbenchLayout";

const ignoreExecutionError = () => {};
const staticSnapshot = () => emptyWorkbench;
const staticSubscribe = () => () => {};

export interface CourseWorkbenchProps {
  course: LoadedCourse;
  client?: CoreClient;
  onExecutionError?: (error: unknown) => void;
  session: WorkspaceEditorSession | null;
  settings: GlobalSettings;
  switching: boolean;
  recentWorkspacePath: string | null;
  onChooseWorkspace: () => void;
  onResumeWorkspace: () => void;
  onRequestTrust: () => void;
  onRevokeTrust?: () => void;
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
  tutorOpen?: boolean;
  tutorSnapshot?: TutorSessionSnapshot | null;
  onTutorSnapshotChange?: (snapshot: TutorSessionSnapshot | null) => void;
  onUpdateSettings?: (patch: UpdateSettingsPatch) => void;
  onOpenRuntimeSettings?: (trigger: HTMLElement) => void;
  onOpenTutorSettings?: (trigger: HTMLElement) => void;
  onOpenTutor?: () => void;
  onCloseTutor?: () => void;
}

function getFileBasename(filePath: string): string {
  const parts = filePath.split(/[/\\]/);
  return parts[parts.length - 1] || filePath;
}

export function CourseWorkbench({
  course, client, onExecutionError = ignoreExecutionError,
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
  tutorOpen = false,
  tutorSnapshot = null,
  onTutorSnapshotChange,
  onUpdateSettings,
  onOpenRuntimeSettings,
  onOpenTutorSettings,
  onOpenTutor,
  onCloseTutor,
}: CourseWorkbenchProps) {
  const verification = useVerification(course.trusted ? client : undefined, session, onExecutionError);
  const [resultsOpen, setResultsOpen] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const [outlineOpen, setOutlineOpen] = useState(false);
  const handleCloseOutline = useCallback(() => setOutlineOpen(false), []);
  const [manualWidthPx, setManualWidthPx] = useState<number | null>(null);
  const [tutorWidthPx, setTutorWidthPx] = useState<number | null>(null);
  const [bottomHeightPx, setBottomHeightPx] = useState<number | null>(null);
  const [terminalMaximized, setTerminalMaximized] = useState(false);
  const [dragAxis, setDragAxis] = useState<"manual" | "tutor" | "terminal" | null>(null);
  const [wbWidth, setWbWidth] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth : 1280
  );
  const [paneCenterHeight, setPaneCenterHeight] = useState(() =>
    typeof window !== "undefined" ? Math.max(400, window.innerHeight - 48) : 720
  );
  const [stripHeight, setStripHeight] = useState(38);
  const [isTutorOverlay, setIsTutorOverlay] = useState(
    () => typeof window !== "undefined" && window.innerWidth < 1280
  );
  useEffect(() => {
    if (typeof window === "undefined") return;
    const syncLayout = () => {
      setIsTutorOverlay(window.innerWidth < 1280);
      const wbRect = wbRef.current?.getBoundingClientRect();
      if (wbRect && wbRect.width > 0) setWbWidth(wbRect.width);
      else setWbWidth(window.innerWidth);
      const centerRect = paneCenterRef.current?.getBoundingClientRect();
      if (centerRect && centerRect.height > 0) setPaneCenterHeight(centerRect.height);
      else setPaneCenterHeight(Math.max(400, window.innerHeight - 48));
      const measuredStrip = stripRef.current?.getBoundingClientRect().height;
      if (measuredStrip && measuredStrip > 0) setStripHeight(measuredStrip);
    };
    syncLayout();
    window.addEventListener("resize", syncLayout);
    let observer: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      observer = new ResizeObserver(syncLayout);
      if (wbRef.current) observer.observe(wbRef.current);
      if (paneCenterRef.current) observer.observe(paneCenterRef.current);
      if (stripRef.current) observer.observe(stripRef.current);
    }
    return () => {
      window.removeEventListener("resize", syncLayout);
      observer?.disconnect();
    };
  }, []);
  const [cwdDialog, setCwdDialog] = useState<{ workbench: CommandWorkbench; command: CommandBlock } | null>(null);
  const workbench = useMemo(() => client && session && course.trusted ? new CommandWorkbench(client, session) : null,
    [client, course.trusted, session]);
  const execution = useSyncExternalStore(workbench?.subscribe ?? staticSubscribe, workbench?.snapshot ?? staticSnapshot);
  useEffect(() => {
    if (workbench) void workbench.loadCwd().catch(onExecutionError);
  }, [course, onExecutionError, workbench]);

  function renderCommand(command: CommandBlock) {
    return <CommandBlockCard key={command.id} command={command} messages={messages} onCopy={onCopyCommand}
      override={execution.cwd[command.id]} disabled={execution.busy || verification.busy || switching || !!verification.progress?.runningChapterId}
      onRun={workbench ? () => void workbench.run({ commandId: command.id }).catch(onExecutionError) : undefined}
      onChangeCwd={workbench ? () => setCwdDialog({ workbench, command }) : undefined} />;
  }

  const wbRef = useRef<HTMLDivElement | null>(null);
  const paneCenterRef = useRef<HTMLElement | null>(null);
  const stripRef = useRef<HTMLDivElement | null>(null);
  const manualScrollRef = useRef<HTMLDivElement | null>(null);
  const manualInnerRef = useRef<HTMLDivElement | null>(null);
  const selectionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const askSeqRef = useRef(0);
  const [selectionState, setSelectionState] = useState<{
    chapterId: string;
    selection: ActiveSelection;
  } | null>(null);
  const [editorCursor, setEditorCursor] = useState<EditorCursorContext>({
    filePath: null,
    startLine: null,
    endLine: null,
  });
  const [pendingAskRequest, setPendingAskRequest] = useState<TutorAskRequest | null>(null);
  const currentChapterIdRef = useRef(activeChapterId);
  useEffect(() => {
    currentChapterIdRef.current = activeChapterId;
  }, [activeChapterId]);

  const handleSelectionChange = useCallback((next: ActiveSelection | null) => {
    if (selectionTimerRef.current !== null) {
      clearTimeout(selectionTimerRef.current);
      selectionTimerRef.current = null;
    }
    if (!next || next.text.trim().length < MIN_SELECTION_CHARS) {
      setSelectionState(null);
      return;
    }
    const capturedChapterId = currentChapterIdRef.current;
    selectionTimerRef.current = setTimeout(() => {
      selectionTimerRef.current = null;
      setSelectionState({ chapterId: capturedChapterId, selection: next });
    }, 35);
  }, []);

  const manualDraggingRef = useRef(false);

  const handleManualSelection = useCallback(() => {
    handleSelectionChange(
      extractDomSelectionWithin(manualInnerRef.current, "manual", true)
    );
  }, [handleSelectionChange]);

  useEffect(() => {
    const handleWindowMouseUp = () => {
      if (!manualDraggingRef.current) return;
      manualDraggingRef.current = false;
      handleManualSelection();
    };
    window.addEventListener("mouseup", handleWindowMouseUp);
    return () => {
      window.removeEventListener("mouseup", handleWindowMouseUp);
      if (selectionTimerRef.current !== null) {
        clearTimeout(selectionTimerRef.current);
      }
    };
  }, [handleManualSelection]);

  const recordLocalMetric = useCallback(
    (event: LocalMetricEvent) => {
      if (!client || !session || !course.trusted) return;
      void client
        .dispatch({
          type: "recordLocalMetric",
          payload: { workspaceId: session.workspace.id, event },
        })
        .catch(() => {});
    },
    [client, course.trusted, session]
  );

  const triggerAskAI = useCallback(
    (
      quote: ContextQuote,
      initialPrompt: string,
      metricAction: MetricSelectionAction = "ask"
    ) => {
      recordLocalMetric({
        kind: "selectionAiClicked",
        zone: quote.source,
        action: metricAction,
      });
      handleSelectionChange(null);
      setPendingAskRequest({
        id: ++askSeqRef.current,
        quote,
        initialPrompt,
      });
      onOpenTutor?.();
    },
    [handleSelectionChange, onOpenTutor, recordLocalMetric]
  );

  const handleConsumeAskRequest = useCallback((consumedId: number) => {
    setPendingAskRequest((current) => (current?.id === consumedId ? null : current));
  }, []);

  const handleSelectionAction = useCallback(
    (selection: ActiveSelection, action: SelectionQuickAction) => {
      triggerAskAI(
        {
          source: selection.zone,
          filePath: selection.filePath ?? null,
          startLine: selection.startLine ?? null,
          endLine: selection.endLine ?? null,
          label: null,
          text: selection.text,
        },
        quickPromptForSelectionAction(action, messages),
        action
      );
    },
    [messages, triggerAskAI]
  );

  const activeIndex = useMemo(() => {
    const idx = course.chapters.findIndex(
      (ch) => ch.chapterId === activeChapterId
    );
    return idx >= 0 ? idx : 0;
  }, [course.chapters, activeChapterId]);

  const activeChapter: Chapter | undefined = course.chapters[activeIndex];
  const chapterProgress = activeChapter ? verification.progress?.chapters[activeChapter.chapterId] : undefined;
  const checkResults = chapterProgress?.lastCheckResults ?? [];
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

  const visibleSelection =
    selectionState && selectionState.chapterId === activeChapterId
      ? selectionState.selection
      : null;

  const isOutlineVisible = outlineOpen && !focusMode;

  const isTutorDocked = tutorOpen && !isTutorOverlay;
  const hasOpenTerminal = execution.terminals.some(({ id }) => id === execution.active);
  const effectiveTerminalMaximized = terminalMaximized && hasOpenTerminal;

  const horizontalLayout = resolveHorizontalWorkbenchWidths({
    containerWidth: wbWidth,
    focusMode,
    isTutorDocked,
    manualWidthPx,
    tutorWidthPx,
  });

  const bottomBounds = computeBottomBounds(paneCenterHeight, stripHeight);
  const defaultBottomHeight = resultsOpen
    ? DEFAULT_VERIFICATION_HEIGHT_PX
    : DEFAULT_TERMINAL_HEIGHT_PX;
  const restoredBottomHeightPx = clamp(
    bottomHeightPx ?? defaultBottomHeight,
    bottomBounds.minPx,
    bottomBounds.maxPx
  );
  const effectiveTerminalHeightPx = effectiveTerminalMaximized
    ? bottomBounds.maximizedPx
    : restoredBottomHeightPx;

  const clampManualWidth = useCallback((clientX: number): number | null => {
    const container = wbRef.current;
    if (!container) return null;
    const rect = container.getBoundingClientRect();
    const containerWidth = rect.width || window.innerWidth || 1280;
    const { minPx, maxPx } = computeManualBounds(
      containerWidth,
      isTutorDocked,
      horizontalLayout.effectiveTutorWidthPx
    );
    const rawPx = Math.round(clientX - rect.left);
    return clamp(rawPx, minPx, maxPx);
  }, [horizontalLayout.effectiveTutorWidthPx, isTutorDocked]);

  const handleSplitterMouseDown = (
    event: React.MouseEvent<HTMLDivElement>
  ) => {
    if (focusMode) return;
    event.preventDefault();
    setDragAxis("manual");

    const handleMove = (moveEvent: MouseEvent) => {
      const clamped = clampManualWidth(moveEvent.clientX);
      if (clamped !== null) {
        setManualWidthPx(clamped);
      }
    };

    const handleUp = () => {
      setDragAxis(null);
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
    const containerWidth = wbRef.current?.getBoundingClientRect().width || wbWidth || 1280;
    const liveLayout = resolveHorizontalWorkbenchWidths({
      containerWidth,
      focusMode,
      isTutorDocked,
      manualWidthPx,
      tutorWidthPx,
    });
    const currentPx = liveLayout.displayManualWidthPx;
    const { minPx, maxPx } = liveLayout.manualBounds;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      setManualWidthPx(clamp(currentPx - 24, minPx, maxPx));
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      setManualWidthPx(clamp(currentPx + 24, minPx, maxPx));
    } else if (event.key === "Home" || event.key === "Enter") {
      event.preventDefault();
      setManualWidthPx(null);
    }
  };

  const clampTutorWidth = useCallback(
    (clientX: number): number | null => {
      const container = wbRef.current;
      if (!container) return null;
      const rect = container.getBoundingClientRect();
      const containerWidth = rect.width || window.innerWidth || 1280;
      const { minPx, maxPx } = computeDockedTutorBounds(
        containerWidth,
        focusMode,
        horizontalLayout.effectiveManualWidthPx
      );
      const rawPx = Math.round(rect.right - clientX);
      return clamp(rawPx, minPx, maxPx);
    },
    [focusMode, horizontalLayout.effectiveManualWidthPx]
  );

  const handleTutorSplitterMouseDown = (
    event: React.MouseEvent<HTMLDivElement>
  ) => {
    if (!isTutorDocked) return;
    event.preventDefault();
    setDragAxis("tutor");

    const handleMove = (moveEvent: MouseEvent) => {
      const clamped = clampTutorWidth(moveEvent.clientX);
      if (clamped !== null) {
        setTutorWidthPx(clamped);
      }
    };

    const handleUp = () => {
      setDragAxis(null);
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
  };

  const handleTutorSplitterKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!isTutorDocked) return;
    const containerWidth = wbRef.current?.getBoundingClientRect().width || wbWidth || 1280;
    const { minPx, maxPx } = computeDockedTutorBounds(
      containerWidth,
      focusMode,
      horizontalLayout.effectiveManualWidthPx
    );
    const currentPx = horizontalLayout.effectiveTutorWidthPx;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      setTutorWidthPx(clamp(currentPx + 24, minPx, maxPx));
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      setTutorWidthPx(clamp(currentPx - 24, minPx, maxPx));
    } else if (event.key === "Home" || event.key === "Enter") {
      event.preventDefault();
      setTutorWidthPx(null);
    }
  };

  const clampBottomHeight = useCallback((clientY: number): number | null => {
    const center = paneCenterRef.current;
    if (!center) return null;
    const rect = center.getBoundingClientRect();
    const measuredHeight = rect.height || paneCenterHeight || 720;
    const measuredStrip = stripRef.current?.getBoundingClientRect().height || 38;
    const { minPx, maxPx } = computeBottomBounds(measuredHeight, measuredStrip);
    const rawPx = Math.round(rect.bottom - clientY - measuredStrip);
    return clamp(rawPx, minPx, maxPx);
  }, [paneCenterHeight]);

  const handleBottomSplitterMouseDown = (
    event: React.MouseEvent<HTMLDivElement>
  ) => {
    event.preventDefault();
    setDragAxis("terminal");
    if (effectiveTerminalMaximized) {
      setTerminalMaximized(false);
    }

    const handleMove = (moveEvent: MouseEvent) => {
      const clamped = clampBottomHeight(moveEvent.clientY);
      if (clamped !== null) {
        setTerminalMaximized(false);
        setBottomHeightPx(clamped);
      }
    };

    const handleUp = () => {
      setDragAxis(null);
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
  };

  const handleBottomSplitterDoubleClick = () => {
    setTerminalMaximized(false);
    setBottomHeightPx(null);
  };

  const handleBottomSplitterKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const measuredHeight = paneCenterRef.current?.getBoundingClientRect().height || paneCenterHeight || 720;
    const measuredStrip = stripRef.current?.getBoundingClientRect().height || 38;
    const { minPx, maxPx } = computeBottomBounds(measuredHeight, measuredStrip);
    const currentPx = resultsOpen ? restoredBottomHeightPx : effectiveTerminalHeightPx;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setTerminalMaximized(false);
      setBottomHeightPx(clamp(currentPx + 24, minPx, maxPx));
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setTerminalMaximized(false);
      setBottomHeightPx(clamp(currentPx - 24, minPx, maxPx));
    } else if (event.key === "Home" || event.key === "Enter") {
      event.preventDefault();
      setTerminalMaximized(false);
      setBottomHeightPx(null);
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

  const effectiveManualWidthPx = horizontalLayout.effectiveManualWidthPx;
  const currentManualDisplayPx = horizontalLayout.displayManualWidthPx;

  const paneLeftStyle =
    !focusMode && effectiveManualWidthPx !== null
      ? { width: `${effectiveManualWidthPx}px` }
      : undefined;

  const wbStyle: CSSProperties | undefined =
    isTutorDocked && tutorWidthPx !== null
      ? ({ "--ai-w": `${horizontalLayout.effectiveTutorWidthPx}px` } as CSSProperties)
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
  const referenceTrigger = course.trusted && session && activeChapter ? (
    <WorkspaceComparisonTrigger key={`${session.workspace.id}:${activeChapter.chapterId}`}
      session={session} chapterId={activeChapter.chapterId} settings={settings} messages={messages}
      disabled={switching} />
  ) : null;
  const activeReflectionsHtml = isThreeStepMode
    ? threeStep.reflectionsHtml
    : (activeChapter?.reflectionsHtml ?? null);

  const visibleStepNumbers = useMemo(() => {
    if (!threeStep) {
      return { step1: null, step2: null, step3: null };
    }
    const hasStep1 = threeStep.step1CommandIds.length > 0;
    const hasStep2 =
      Boolean(threeStep.step2Html) || Boolean(threeStep.step2EntryFile) || Boolean(activeChapter?.hasReferenceHint);
    const hasStep3 =
      threeStep.step3Checks.length > 0 ||
      Boolean(threeStep.step3IntroHtml) ||
      Boolean(threeStep.requiresDaemon);

    const step1 = hasStep1 ? 1 : null;
    const step2 = hasStep2 ? (step1 ?? 0) + 1 : null;
    const step3 = hasStep3 ? (step2 ?? step1 ?? 0) + 1 : null;
    return { step1, step2, step3 };
  }, [activeChapter?.hasReferenceHint, threeStep]);

  const secondaryCommands = useMemo(() => {
    if (!threeStep) return [];
    return threeStep.secondaryCommandIds
      .map((id) => commandById.get(id))
      .filter((cmd): cmd is CommandBlock => cmd !== undefined);
  }, [threeStep, commandById]);

  const reloadFileLabel = pendingReload?.changedPaths[0]
    ? getFileBasename(pendingReload.changedPaths[0])
    : course.title;

  const pendingConfirmation = execution.confirmation;
  const isResizingX = dragAxis === "manual" || dragAxis === "tutor";
  const isResizingY = dragAxis === "terminal";

  return (
    <section
      ref={wbRef}
      className={`wb${focusMode ? " is-focus" : ""}${isResizingX ? " is-resizing" : ""}${isResizingY ? " is-resizing-y" : ""}${isTutorDocked ? " has-tutor-docked" : ""}`}
      style={wbStyle}
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
          onClose={handleCloseOutline}
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

        <div
          ref={manualScrollRef}
          className="manual-scroll"
          onMouseDown={() => {
            manualDraggingRef.current = true;
          }}
          onMouseUp={() => {
            manualDraggingRef.current = false;
            handleManualSelection();
          }}
          onKeyUp={handleManualSelection}
          onScroll={() => handleSelectionChange(null)}
        >
          {activeChapter ? (
            <div ref={manualInnerRef} className="manual-inner">
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
                          renderCommand(cmd)
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
                      {referenceTrigger}
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

                  {visibleStepNumbers.step2 === null && course.source.kind !== "localMarkdown" && referenceTrigger}

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

                        {threeStep.step3Checks.map((check) => {
                          const result = checkResults.find(r=>r.checkId===check.id);
                          const isFailed = result?.status === "failed" || result?.status === "timedOut";
                          return <div key={check.id} className="v-item"><div className="v-row">
                            {check.kind === "selfCheck" ? <input type="checkbox" className="arc-checkbox" aria-label={check.title}
                              disabled={!session || switching || verification.busy}
                              checked={chapterProgress?.selfChecks[check.id] ?? false}
                              onChange={event=>session && void verification.update({type:"setSelfCheck",payload:{workspaceId:session.workspace.id,chapterId:activeChapter.chapterId,checkId:check.id,checked:event.target.checked}})} />
                              : <span className={`badge badge--${result?.status === "passed" ? "success" : result?.status === "failed" || result?.status === "timedOut" ? "danger" : "neutral"}`}>{result ? messages.verification.status[result.status] : t.checkKinds[check.kind]}</span>}
                            <span className="v-text">{check.title}{check.kind === "selfCheck" && check.required && <span className="badge badge--neutral">{messages.verification.required}</span>}</span>
                            {result && <span className="mono">{result.durationMs} ms</span>}
                            {isFailed && result && (
                              <button
                                type="button"
                                className="btn btn--ai-soft btn--xs btn--pill"
                                aria-label={interpolate(messages.tutor.context.askAiCheckFailure, { title: check.title })}
                                data-testid={`step3-check-ask-ai-${check.id}`}
                                onClick={() =>
                                  triggerAskAI(
                                    {
                                      source: "checkFailure",
                                      label: check.title,
                                      text: formatCheckFailureQuote(check, result, messages, settings.locale),
                                    },
                                    messages.tutor.quickPrompts.checkFailure,
                                    "diagnoseError"
                                  )
                                }
                              >
                                <SparklesIcon size={11} />
                                <span>{messages.tutor.context.askAiAction}</span>
                              </button>
                            )}
                          </div></div>;
                        })}
                        <div className="verification-actions">
                          {threeStep.step3Checks.some(c=>c.kind!=="selfCheck") ? <button className="btn btn--primary btn--sm" type="button"
                            disabled={!session || switching || verification.busy || !!verification.progress?.runningChapterId}
                            onClick={()=>{setResultsOpen(true); void verification.run(activeChapter.chapterId);}}>
                            {verification.progress?.runningChapterId === activeChapter.chapterId ? messages.verification.running : messages.verification.run}
                          </button> : <button className="btn btn--secondary btn--sm" type="button"
                            disabled={!session || switching || verification.busy || chapterProgress?.completionState === "completed"}
                            onClick={()=>session && void verification.update({type:"completeExercise",payload:{workspaceId:session.workspace.id,chapterId:activeChapter.chapterId}})}>{messages.verification.complete}</button>}
                          {chapterProgress?.completionState === "completed" && <span className="verification-completed">{messages.verification.completed}</span>}
                        </div>
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
                          renderCommand(cmd)
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

                  {activeChapter.hasReferenceHint || (course.source.kind !== "localMarkdown" && activeChapter.kind === "exercise") ? referenceTrigger : null}
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
                      renderCommand(cmd)
                    );
                  })}
                  {activeChapter.kind === "exercise" && <div className="verification-actions">
                    <button type="button" className="btn btn--secondary btn--sm" disabled={!session || switching || chapterProgress?.completionState === "completed"}
                      onClick={()=>session && void verification.update({type:"completeExercise",payload:{workspaceId:session.workspace.id,chapterId:activeChapter.chapterId}})}>{messages.verification.complete}</button>
                    {chapterProgress?.completionState === "completed" && <span className="verification-completed">{messages.verification.completed}</span>}
                  </div>}
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
                    <div className="reflect-actions">
                      <button
                        type="button"
                        className="btn btn--ai-soft btn--xs btn--pill"
                        data-testid="reflections-ask-ai"
                        onClick={() =>
                          triggerAskAI(
                            {
                              source: "reflection",
                              label: t.reflectionsTitle,
                              text: htmlToPlainText(activeReflectionsHtml),
                            },
                            messages.tutor.quickPrompts.reflection
                          )
                        }
                      >
                        <SparklesIcon size={12} />
                        <span>{messages.tutor.context.askAiReflection}</span>
                      </button>
                    </div>
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

      {/* Draggable Vertical Splitter: Manual <-> Right Workbench */}
      <div
        className={`split-x split-x--manual${dragAxis === "manual" ? " is-drag" : ""}`}
        role="separator"
        aria-orientation="vertical"
        aria-label={t.splitterAriaLabel}
        aria-valuenow={currentManualDisplayPx}
        aria-valuemin={horizontalLayout.manualBounds.minPx}
        aria-valuemax={horizontalLayout.manualBounds.maxPx}
        tabIndex={focusMode ? -1 : 0}
        title={t.splitterTitle}
        data-testid="workbench-splitter"
        onMouseDown={handleSplitterMouseDown}
        onDoubleClick={handleSplitterDoubleClick}
        onKeyDown={handleSplitterKeyDown}
      />

      <section ref={paneCenterRef} className="pane-center" aria-label={t.stageAriaLabel}>
        {stage ?? (session && workbench ? (
          <>
            <WorkspaceEditor
              key={session.workspace.id}
              session={session}
              settings={settings}
              messages={messages}
              switching={switching}
              tutorOpen={isTutorDocked}
              onChangeWorkspace={onReturnToDraft ?? onChooseWorkspace}
              workspaceActionLabel={onReturnToDraft ? messages.coursePack.returnDraft : undefined}
              workspaceLabel={onReturnToDraft ? messages.coursePack.trialTitle : undefined}
              onOpenRuntimeSettings={onOpenRuntimeSettings}
              onUpdateSettings={onUpdateSettings}
              onFileTreeOpen={() => recordLocalMetric({ kind: "fileTreeOpen" })}
              onRevokeTrust={course.isBuiltin ? undefined : onRevokeTrust}
              onSelectionChange={handleSelectionChange}
              onCursorContextChange={setEditorCursor}
            />
            {(hasOpenTerminal || resultsOpen) && (
              <div
                className={`split-y${dragAxis === "terminal" ? " is-drag" : ""}`}
                role="separator"
                aria-orientation="horizontal"
                aria-label={t.terminalSplitterAriaLabel}
                aria-valuenow={resultsOpen ? restoredBottomHeightPx : effectiveTerminalHeightPx}
                aria-valuemin={bottomBounds.minPx}
                aria-valuemax={bottomBounds.maxPx}
                tabIndex={0}
                title={t.terminalSplitterTitle}
                data-testid="terminal-splitter"
                onMouseDown={handleBottomSplitterMouseDown}
                onDoubleClick={handleBottomSplitterDoubleClick}
                onKeyDown={handleBottomSplitterKeyDown}
              />
            )}
            <div ref={stripRef} className="verification-strip">
              <div className="verification-tabs" role="tablist" aria-label={messages.verification.results} onKeyDown={event=>{
                if (!["ArrowLeft","ArrowRight","Home","End"].includes(event.key) || (event.target as HTMLElement).getAttribute("role") !== "tab") return;
                event.preventDefault(); const next = event.key === "Home" ? false : event.key === "End" ? true : !resultsOpen;
                setResultsOpen(next); document.getElementById(next ? "verification-results-tab" : "execution-terminal-tab")?.focus();
              }}>
                <button type="button" role="tab" id="execution-terminal-tab" aria-selected={!resultsOpen} tabIndex={resultsOpen ? -1 : 0} aria-controls="execution-terminal-panel" onClick={()=>setResultsOpen(false)}>{messages.execution.terminal}</button>
                <button type="button" role="tab" id="verification-results-tab" aria-selected={resultsOpen} tabIndex={resultsOpen ? 0 : -1} aria-controls="verification-results-panel" onClick={()=>setResultsOpen(true)}>{messages.verification.results} <span className="mono">{checkResults.filter(r=>r.status==="passed").length}/{checkResults.length}</span></button>
              </div>
              <div className="verification-record-actions">
                <button className="btn btn--ghost btn--xs" type="button" onClick={()=>void verification.exportRecord()}>{messages.verification.export}</button>
                <button className="btn btn--ghost btn--xs" type="button" onClick={()=>setResetOpen(true)}>{messages.verification.reset}</button>
              </div>
            </div>
            <div
              role="tabpanel"
              id="verification-results-panel"
              aria-labelledby="verification-results-tab"
              hidden={!resultsOpen}
              className="verification-panel"
              style={bottomHeightPx !== null ? { height: `${restoredBottomHeightPx}px` } : undefined}
            >
              <CheckResults
                checks={activeChapter?.threeStep?.step3Checks ?? []}
                results={checkResults}
                messages={messages}
                locale={settings.locale}
                onAskCheck={(check, result) =>
                  triggerAskAI(
                    {
                      source: "checkFailure",
                      label: check?.title ?? result.checkId,
                      text: formatCheckFailureQuote(check, result, messages, settings.locale),
                    },
                    messages.tutor.quickPrompts.checkFailure,
                    "diagnoseError"
                  )
                }
                onAskAll={() => {
                  const specs = activeChapter?.threeStep?.step3Checks ?? [];
                  const failing = checkResults.filter(
                    (r) => r.status === "failed" || r.status === "timedOut"
                  );
                  const targets = failing.length > 0 ? failing : checkResults;
                  const summaryText = targets
                    .map((result) =>
                      formatCheckFailureQuote(
                        specs.find((s) => s.id === result.checkId),
                        result,
                        messages,
                        settings.locale
                      )
                    )
                    .join("\n\n");
                  triggerAskAI(
                    {
                      source: "verificationPanel",
                      label: messages.verification.results,
                      text: summaryText,
                    },
                    messages.tutor.quickPrompts.verificationPanel,
                    "diagnoseError"
                  );
                }}
              />
            </div>
            <div
              role="tabpanel"
              className={`execution-terminal-container${effectiveTerminalMaximized ? " execution-terminal-container--maximized" : ""}`}
              id="execution-terminal-panel"
              aria-labelledby="execution-terminal-tab"
              hidden={resultsOpen}
              style={hasOpenTerminal ? { height: `${effectiveTerminalHeightPx}px` } : undefined}
            >
            <TerminalPanel
              key={`terminal-${session.workspace.id}`}
              workbench={workbench}
              state={execution}
              messages={messages}
              settings={settings}
              disabled={switching}
              maximized={effectiveTerminalMaximized}
              onToggleMaximize={() => setTerminalMaximized((value) => !value)}
              onCloseLastTerminal={() => setTerminalMaximized(false)}
              onError={onExecutionError}
              onSelectionChange={handleSelectionChange}
              onAskAI={({ port, text }) =>
                triggerAskAI(
                  {
                    source: "portConflict",
                    label:
                      port !== null
                        ? interpolate(messages.execution.port, { port })
                        : messages.execution.portConflict,
                    text,
                  },
                  messages.tutor.quickPrompts.portConflict,
                  "diagnoseError"
                )
              }
            />
            </div>
          </>
        ) : (
          <>
            <div className="stage-bar">
              <div className="stage-bar-left">
                <span className="badge badge--neutral">{sourceBadge}</span>
                {!course.trusted && <span className="badge badge--warning">{t.readOnlyPreviewBadge}</span>}
              </div>
              {course.trusted && !course.isBuiltin && onRevokeTrust && (
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
                {onReturnToDraft && <button type="button" className="btn btn--secondary btn--sm"
                  disabled={switching} onClick={onReturnToDraft}>{messages.coursePack.returnDraft}</button>}
              </div>
            </div>
          </>
        ))}
      </section>
      {isTutorDocked && onCloseTutor && onTutorSnapshotChange && onUpdateSettings && onOpenTutorSettings && (
        <div
          className={`split-x split-x--tutor${dragAxis === "tutor" ? " is-drag" : ""}`}
          role="separator"
          aria-orientation="vertical"
          aria-label={t.tutorSplitterAriaLabel}
          aria-valuenow={horizontalLayout.effectiveTutorWidthPx}
          aria-valuemin={horizontalLayout.tutorBounds.minPx}
          aria-valuemax={horizontalLayout.tutorBounds.maxPx}
          tabIndex={0}
          title={t.tutorSplitterTitle}
          data-testid="tutor-splitter"
          onMouseDown={handleTutorSplitterMouseDown}
          onDoubleClick={() => setTutorWidthPx(null)}
          onKeyDown={handleTutorSplitterKeyDown}
        />
      )}
      {tutorOpen && onCloseTutor && onTutorSnapshotChange && onUpdateSettings && onOpenTutorSettings && (
        <TutorDrawer
          open={tutorOpen}
          overlay={isTutorOverlay}
          client={client}
          course={course}
          activeChapterId={activeChapter?.chapterId ?? ""}
          session={session}
          editorCursor={editorCursor}
          verificationProgress={verification.progress}
          terminalRevision={execution.outputVersion}
          askRequest={pendingAskRequest}
          onConsumeAskRequest={handleConsumeAskRequest}
          settings={settings}
          messages={messages}
          snapshot={tutorSnapshot}
          onSnapshotChange={onTutorSnapshotChange}
          onUpdateSettings={onUpdateSettings}
          onOpenTutorSettings={onOpenTutorSettings}
          onRequestTrust={onRequestTrust}
          onChooseWorkspace={onChooseWorkspace}
          onClose={() => {
            setPendingAskRequest(null);
            onCloseTutor();
          }}
          onError={onExecutionError}
        />
      )}
      <SelectionAskPill
        selection={visibleSelection}
        messages={messages}
        onAction={handleSelectionAction}
        onDismiss={() => handleSelectionChange(null)}
      />
      {verification.confirmation && <CommandConfirmationDialog detail={verification.confirmation.detail} messages={messages}
        onCancel={() => {
          recordLocalMetric({ kind: "cmdRiskModalCancelled", source: "verification" });
          verification.dismiss();
        }}
        onConfirm={()=>{ const pending=verification.confirmation!; void verification.run(pending.chapterId,pending.detail.token); }} />}
      {resetOpen && session && <WorkspaceDialog title={messages.verification.resetTitle} detail={messages.verification.resetDetail}
        cancel={messages.workspace.cancel} onCancel={()=>setResetOpen(false)}>
        <button className="btn btn--danger btn--sm" type="button" onClick={()=>{setResetOpen(false); void verification.update({type:"resetProgress",payload:{workspaceId:session.workspace.id}});}}>{messages.verification.reset}</button>
      </WorkspaceDialog>}
      {workbench && pendingConfirmation && (
        <CommandConfirmationDialog detail={pendingConfirmation.detail} messages={messages}
          onCancel={() => {
            recordLocalMetric({ kind: "cmdRiskModalCancelled", source: "commandBlock" });
            workbench.dismissConfirmation();
          }}
          onConfirm={() => {
            const { request, detail } = pendingConfirmation;
            void workbench.run({ ...request, confirmation: detail.token }).catch(onExecutionError);
          }} />
      )}
      {workbench && cwdDialog?.workbench === workbench && (
        <CommandCwdDialog key={cwdDialog.command.id} command={cwdDialog.command} override={execution.cwd[cwdDialog.command.id]}
          messages={messages} locale={settings.locale} onCancel={() => setCwdDialog(null)}
          onSave={(value) => workbench.setCwd(cwdDialog.command.id, value)} />
      )}
    </section>
  );
}
