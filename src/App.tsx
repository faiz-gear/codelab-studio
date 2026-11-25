import { useCallback, useEffect, useRef, useState } from "react";
import {
  closeActiveCourse,
  dispatchPreflight,
  expectResponse,
  fetchBootstrap,
  openCourse,
  reloadActiveCourse,
  updateSettings,
  type CoreClient,
} from "./core/client";
import {
  CoreCommandError,
  courseSourceKey,
  courseSourceLocation,
  type CourseDraft,
  type TrustSource,
  openRequestFromSource,
  type CourseSourceChangedEvent,
  type CourseSummary,
  type DomainError,
  type GlobalSettings,
  type LoadedCourse,
  type LocalPathPickKind,
  type OpenCourseRequest,
  type PreflightCommand,
  type PreflightReport,
  type RecentWorkspace,
  type UpdateSettingsPatch,
  type WorkspaceSummary,
} from "./core/contracts";
import { DraftWorkbench, type DraftNavigation } from "./authoring/DraftWorkbench";
import { AppearanceDrawer } from "./components/AppearanceDrawer";
import { CourseWorkbench } from "./components/CourseWorkbench";
import { ImportCourseModal, type DraftOpenCommand, type ImportMode } from "./components/ImportCourseModal";
import { RuntimeSettingsDialog } from "./components/RuntimeSettingsDialog";
import {
  ToastStack,
  type ToastMessage,
  type ToastTone,
} from "./components/ToastStack";
import { TopBar } from "./components/TopBar";
import { WelcomeHome } from "./components/WelcomeHome";
import { WorkspaceDialog } from "./workspace/WorkspaceDialog";
import { domainError, WorkspaceEditorSession } from "./workspace/editorSession";
import { useWorkspaceNavigation } from "./workspace/useWorkspaceNavigation";
import { formatDomainError, getMessages, interpolate } from "./i18n";
import {
  applySettingsToDocument,
  buildThemeConfigSnippet,
  readCachedSettings,
} from "./theme/applyTheme";

export interface AppProps {
  client: CoreClient;
}

const TOAST_TTL_MS = 3800;

function pickInitialChapterId(course: LoadedCourse): string {
  const firstExercise = course.chapters.find((ch) => ch.kind === "exercise");
  return firstExercise?.chapterId ?? course.chapters[0]?.chapterId ?? "";
}

function toCourseSummary(course: LoadedCourse): CourseSummary {
  return {
    courseId: course.courseId,
    title: course.title,
    description: course.description,
    language: course.language,
    source: course.source,
    totalChapters: course.chapters.length,
    totalExercises: course.totalExercises,
    isBuiltin: course.isBuiltin,
  };
}

export function App({ client }: AppProps) {
  const [settings, setSettings] = useState<GlobalSettings>(readCachedSettings);
  const [recents, setRecents] = useState<RecentWorkspace[]>([]);
  const [builtinCourses, setBuiltinCourses] = useState<CourseSummary[]>([]);
  const [importedCourses, setImportedCourses] = useState<CourseSummary[]>([]);
  const [activeCourse, setActiveCourse] = useState<LoadedCourse | null>(null);
  const [workspace, setWorkspace] = useState<WorkspaceEditorSession | null>(null);
  const [resumePath, setResumePath] = useState<string | null>(null);
  const [trustPrompt, setTrustPrompt] = useState(false);
  const workspaceRef = useRef<WorkspaceEditorSession | null>(null);
  const [activeChapterId, setActiveChapterId] = useState<string>("");
  const [focusMode, setFocusMode] = useState(false);
  const [pendingReload, setPendingReload] =
    useState<CourseSourceChangedEvent | null>(null);
  const [reloadingCourse, setReloadingCourse] = useState(false);

  const [appearanceOpen, setAppearanceOpen] = useState(false);
  const [runtimeSettingsOpen, setRuntimeSettingsOpen] = useState(false);
  const [preflight, setPreflight] = useState<PreflightReport | null>(null);
  const [preflightError, setPreflightError] = useState<DomainError | null>(null);
  const [importMode, setImportMode] = useState<ImportMode | null>(null);
  const [draft, setDraft] = useState<CourseDraft | null>(null);
  const [trial, setTrial] = useState(false);
  const [draftGuard, setDraftGuard] = useState<DraftNavigation | null>(null);
  const authoring = draft !== null && !trial;
  const [importSubmitting, setImportSubmitting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const appearanceTriggerRef = useRef<HTMLButtonElement | null>(null);
  const runtimeSettingsTrigger = useRef<HTMLElement | null>(null);
  const nextToastId = useRef(1);
  const localeRef = useRef(settings.locale);
  const activeCourseRef = useRef<LoadedCourse | null>(null);

  useEffect(() => {
    localeRef.current = settings.locale;
  }, [settings.locale]);

  const pushToast = useCallback((text: string, tone: ToastTone = "info") => {
    const id = nextToastId.current++;
    setToasts((prev) => [...prev.slice(-1), { id, tone, text }]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
    }, TOAST_TTL_MS);
  }, []);

  const handleExecutionError = useCallback((error: unknown) => {
    pushToast(formatDomainError(domainError(error), localeRef.current), "err");
  }, [pushToast]);

  const applyLoadedCourse = useCallback((loaded: LoadedCourse, addToLibrary = true) => {
    const prevCourse = activeCourseRef.current;
    const isSameCourse = prevCourse && courseSourceKey(prevCourse.source) === courseSourceKey(loaded.source);

    if (!isSameCourse) {
      workspaceRef.current = null;
      setWorkspace(null);
      setResumePath(null);
    }
    activeCourseRef.current = loaded;
    setActiveCourse(loaded);
    setPendingReload(null);
    setActiveChapterId((currentId) => {
      if (
        isSameCourse &&
        currentId &&
        loaded.chapters.some((ch) => ch.chapterId === currentId)
      ) {
        return currentId;
      }
      return pickInitialChapterId(loaded);
    });

    if (addToLibrary && !loaded.isBuiltin) {
      const summary = toCourseSummary(loaded);
      const key = courseSourceKey(summary.source);
      setImportedCourses((prev) => [
        summary,
        ...prev.filter((item) => courseSourceKey(item.source) !== key),
      ]);
    }
  }, []);

  const acceptPreflight = useCallback((report: PreflightReport) => {
    // A streamed update can arrive ahead of the response to the command that started it.
    setPreflight((current) => !current || report.revision > current.revision ? report : current);
  }, []);

  const runPreflight = useCallback(async (command: PreflightCommand) => {
    setPreflightError(null);
    try {
      acceptPreflight(await dispatchPreflight(client, command));
    } catch (err) {
      setPreflightError(err instanceof CoreCommandError ? err.domainError : { code: "UNEXPECTED_ERROR" });
    }
  }, [acceptPreflight, client]);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | null = null;

    void (async () => {
      try {
        unsubscribe = await client.subscribe((event) => {
          if (!active) return;
          if (event.type === "settingsUpdated") {
            setSettings(event.payload);
            applySettingsToDocument(event.payload, {
              disableTransitions: true,
            });
          } else if (event.type === "recentsUpdated") {
            setRecents(event.payload);
          } else if (event.type === "preflightUpdated") {
            acceptPreflight(event.payload);
          } else if (event.type === "workspaceFilesChanged") {
            const editor = workspaceRef.current;
            if (editor?.workspace.id === event.payload.workspaceId) {
              void editor.refresh(event.payload.paths);
            }
          } else if (event.type === "courseTrustChanged") {
            const course = activeCourseRef.current;
            if (course && courseSourceKey(course.source) === courseSourceKey(event.payload.source)) {
              const updated = { ...course, trusted: event.payload.trusted };
              activeCourseRef.current = updated;
              setActiveCourse(updated);
              workspaceRef.current?.setTrusted(updated.trusted);
            }
          } else if (event.type === "courseSourceChanged") {
            if (activeCourseRef.current?.courseId === event.payload.courseId) {
              setPendingReload(event.payload);
            }
          }
        });

        if (!active) {
          unsubscribe();
          return;
        }
        void runPreflight({ type: "refreshPreflight" });
        const bootstrap = await fetchBootstrap(client);
        if (!active) return;
        setSettings(bootstrap.settings);
        setRecents(bootstrap.recents);
        setBuiltinCourses(bootstrap.builtinCourses);
        applySettingsToDocument(bootstrap.settings, {
          disableTransitions: true,
        });

        for (const notice of bootstrap.recoveryNotices) {
          pushToast(
            formatDomainError(
              { code: notice.errorCode, details: { store: notice.store } },
              bootstrap.settings.locale
            ),
            "warn"
          );
        }
      } catch (err) {
        if (!active) return;
        if (err instanceof CoreCommandError) {
          pushToast(formatDomainError(err.domainError, localeRef.current), "err");
        }
      }
    })();

    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [acceptPreflight, client, pushToast, runPreflight]);

  // Global ⌘/Ctrl + B Focus Mode shortcut while inside the Workbench
  useEffect(() => {
    if (!activeCourse && !authoring) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (importMode || appearanceOpen || runtimeSettingsOpen || event.defaultPrevented || document.querySelector("dialog[open]")) return;
      if (
        (event.metaKey || event.ctrlKey) &&
        !event.altKey &&
        event.key.toLowerCase() === "b"
      ) {
        event.preventDefault();
        setFocusMode((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeCourse, authoring, importMode, appearanceOpen, runtimeSettingsOpen]);

  const messages = getMessages(settings.locale);
  const runtimePanel = {
    report: preflight,
    error: preflightError,
    messages,
    locale: settings.locale,
    onCommand: (command: PreflightCommand) => void runPreflight(command),
  };

  function openRuntimeSettings(trigger: HTMLElement) {
    runtimeSettingsTrigger.current = trigger;
    setRuntimeSettingsOpen(true);
  }

  function closeRuntimeSettings() {
    setRuntimeSettingsOpen(false);
    queueMicrotask(() => runtimeSettingsTrigger.current?.focus());
  }

  const handleCloseAppearance = useCallback(() => {
    setAppearanceOpen(false);
    queueMicrotask(() => {
      appearanceTriggerRef.current?.focus();
    });
  }, []);

  const handleUpdateSettings = useCallback(
    async (patch: UpdateSettingsPatch) => {
      try {
        const updated = await updateSettings(client, patch);
        setSettings(updated);
        applySettingsToDocument(updated, { disableTransitions: true });
      } catch (err) {
        if (err instanceof CoreCommandError) {
          pushToast(formatDomainError(err.domainError, localeRef.current), "err");
        }
      }
    },
    [client, pushToast]
  );

  const handleCopyThemeConfig = useCallback(async () => {
    const snippet = buildThemeConfigSnippet(settings);
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("clipboard unavailable");
      }
      await navigator.clipboard.writeText(snippet);
      pushToast(
        interpolate(messages.appearance.copiedToast, {
          theme: settings.theme,
          accent: settings.accent,
        }),
        "ok"
      );
    } catch {
      pushToast(messages.appearance.copyFailedToast, "warn");
    }
  }, [
    messages.appearance.copiedToast,
    messages.appearance.copyFailedToast,
    pushToast,
    settings,
  ]);

  const navigation = useWorkspaceNavigation(workspace ?? (authoring ? draftGuard : null), (error) => {
    pushToast(formatDomainError(domainError(error), localeRef.current), "err");
  });

  function applyBoundWorkspace(bound: WorkspaceSummary | null) {
    const editor = bound ? new WorkspaceEditorSession(client, bound) : null;
    workspaceRef.current = editor;
    setWorkspace(editor);
    for (const notice of bound?.recoveryNotices ?? []) {
      pushToast(formatDomainError({ code: notice.errorCode, details: { store: notice.store } }, settings.locale), "warn");
    }
  }

  function handleOpenCourseRequest(request: OpenCourseRequest, fromImportModal = false) {
    return navigation.navigate(async () => {
      if (fromImportModal) {
        setImportSubmitting(true);
        setImportError(null);
      }
      try {
        const loaded = await openCourse(client, request);
        applyLoadedCourse(loaded);
        if (trial) applyBoundWorkspace(null);
        setDraft(null);
        setTrial(false);
        setFocusMode(false);
        if (fromImportModal) setImportMode(null);
      } catch (error) {
        if (!fromImportModal) throw error;
        setImportError(formatDomainError(domainError(error), localeRef.current));
      } finally {
        if (fromImportModal) setImportSubmitting(false);
      }
    }, Boolean(draft) || !activeCourse || courseSourceKey(activeCourse.source) !== courseSourceKey(request));
  }

  function handleSelectCourseSummary(summary: CourseSummary) {
    void handleOpenCourseRequest(openRequestFromSource(summary.source));
  }

  async function bindWorkspace(path: string) {
    const response = await client.dispatch({ type: "bindWorkspace", payload: { path } });
    applyBoundWorkspace(expectResponse(response, "workspaceBound").payload);
    setResumePath(null);
  }

  async function chooseWorkspace() {
    try {
      const course = activeCourseRef.current;
      const path = await client.pickLocalPath?.("workspaceDir");
      if (path && course === activeCourseRef.current && path !== workspaceRef.current?.workspace.root) {
        await navigation.navigate(() => bindWorkspace(path));
      }
    } catch (error) {
      pushToast(formatDomainError(domainError(error), localeRef.current), "err");
    }
  }

  function handleOpenRecentWorkspace(path: string) {
    void navigation.navigate(async () => {
      const response = await client.dispatch({ type: "openRecentWorkspace", payload: { path } });
      const opened = expectResponse(response, "recentWorkspaceOpened").payload;
      applyLoadedCourse(opened.course);
      applyBoundWorkspace(opened.workspace);
      setDraft(null);
      setTrial(false);
      setResumePath(opened.workspace ? null : path);
      setFocusMode(false);
    });
  }

  function setCourseTrust(trust: boolean) {
    const course = activeCourseRef.current;
    if (!course || course.source.kind === "builtin") return;
    const source: TrustSource = {
      sourceKind: course.source.kind === "gitPack" ? "gitRepo" : "localPath",
      source: courseSourceLocation(course.source),
    };
    void navigation.navigate(async () => {
      try {
        const response = await client.dispatch({ type: trust ? "grantTrust" : "revokeTrust", payload: source });
        expectResponse(response, trust ? "trustState" : "trustEntries");
      } finally {
        setTrustPrompt(false);
      }
    }, false);
  }

  function handleGoHome() {
    if (!activeCourseRef.current && !draft) {
      window.scrollTo({ top: 0 });
      return;
    }
    void navigation.navigate(async () => {
      await closeActiveCourse(client);
      activeCourseRef.current = null;
      applyBoundWorkspace(null);
      setActiveCourse(null);
      setTrial(false);
      setDraft(null);
      setResumePath(null);
      setActiveChapterId("");
      setPendingReload(null);
      setFocusMode(false);
    });
  }

  function handleReloadCourse() {
    return navigation.navigate(async () => {
      setReloadingCourse(true);
      try {
        const reloaded = activeCourseRef.current?.source.kind === "gitPack"
          ? expectResponse(await client.dispatch({ type: "refreshGitCourse" }), "loadedCourse").payload
          : await reloadActiveCourse(client);
        applyLoadedCourse(reloaded);
        pushToast(interpolate(messages.workbench.reloadedToast, { title: reloaded.title }), "ok");
      } finally {
        setReloadingCourse(false);
      }
    }, false);
  }

  function handleOpenDraft(command: DraftOpenCommand) {
    return navigation.navigate(async () => {
      setImportSubmitting(true);
      setImportError(null);
      try {
        const response = await client.dispatch(command);
        setDraft(expectResponse(response, "courseDraft").payload);
        setTrial(false);
        activeCourseRef.current = null;
        setActiveCourse(null);
        applyBoundWorkspace(null);
        setFocusMode(false);
        setPendingReload(null);
        setImportMode(null);
      } catch (error) {
        setImportError(formatDomainError(domainError(error), localeRef.current));
      } finally {
        setImportSubmitting(false);
      }
    });
  }

  function handleStartTrial(id: number) {
    return navigation.navigate(async () => {
      const response = await client.dispatch({ type: "startTrialWorkspace", payload: { draftId: id } });
      const opened = expectResponse(response, "trialWorkspaceOpened").payload;
      applyLoadedCourse(opened.course, false);
      applyBoundWorkspace(opened.workspace);
      setTrial(true);
      setFocusMode(false);
      setDraft((current) => current?.id === id ? {
        ...current, preview: { course: { ...opened.course, trusted: false }, diagnostics: [] },
      } : current);
    }, false);
  }

  function handleReturnToDraft() {
    return navigation.navigate(async () => {
      await closeActiveCourse(client);
      activeCourseRef.current = null;
      setActiveCourse(null);
      applyBoundWorkspace(null);
      setTrial(false);
      setFocusMode(false);
    });
  }

  function showImport(mode: ImportMode) {
    setImportError(null);
    setImportMode(mode);
  }

  const handleCopyCommand = useCallback(
    async (commandText: string) => {
      try {
        if (!navigator.clipboard?.writeText) {
          throw new Error("clipboard unavailable");
        }
        await navigator.clipboard.writeText(commandText);
        pushToast(messages.workbench.copiedCommandToast, "ok");
      } catch {
        pushToast(messages.workbench.copyFailedToast, "warn");
      }
    },
    [
      messages.workbench.copiedCommandToast,
      messages.workbench.copyFailedToast,
      pushToast,
    ]
  );

  const handleBrowsePath = useCallback(
    async (kind: LocalPathPickKind): Promise<string | null> => {
      if (!client.pickLocalPath) {
        return null;
      }
      return client.pickLocalPath(kind);
    },
    [client]
  );

  const handlePlaceholderAction = useCallback(
    (target: string) => {
      pushToast(
        interpolate(messages.welcome.placeholderNotice, { target }),
        "info"
      );
    },
    [messages.welcome.placeholderNotice, pushToast]
  );

  return (
    <div className="app-shell">
      <a href="#mainView" className="skip-link">
        {messages.skipToMain}
      </a>

      <TopBar
        messages={messages}
        builtinCourses={builtinCourses}
        importedCourses={importedCourses}
        activeCourse={authoring ? draft.preview.course : activeCourse}
        completedExercises={workspace ? recents.find((item) => item.workspacePath === workspace.workspace.root)?.completedExercises : undefined}
        focusMode={focusMode}
        appearanceOpen={appearanceOpen}
        appearanceTriggerRef={appearanceTriggerRef}
        onGoHome={handleGoHome}
        onSelectCourse={handleSelectCourseSummary}
        onOpenImportModal={() => showImport("localMarkdown")}
        onToggleFocusMode={() => setFocusMode((prev) => !prev)}
        onOpenAppearance={() => setAppearanceOpen(true)}
        runtimePanel={runtimePanel}
        runtimeSettingsOpen={runtimeSettingsOpen}
        onOpenRuntimeSettings={openRuntimeSettings}
      />

      <main className="view-host" id="mainView">
        {authoring ? (
          <DraftWorkbench key={draft.id} draft={draft} client={client} settings={settings}
            messages={messages} switching={navigation.switching} focusMode={focusMode}
            onToggleFocusMode={() => setFocusMode((prev) => !prev)} onGuard={setDraftGuard}
            onTrial={() => handleStartTrial(draft.id)} onCopyCommand={(text) => void handleCopyCommand(text)} />
        ) : activeCourse ? (
          <CourseWorkbench
            client={client}
            onExecutionError={handleExecutionError}
            course={activeCourse}
            session={workspace}
            settings={settings}
            switching={navigation.switching}
            recentWorkspacePath={resumePath}
            onChooseWorkspace={() => void chooseWorkspace()}
            onResumeWorkspace={() => {
              if (resumePath) void navigation.navigate(() => bindWorkspace(resumePath));
            }}
            onRequestTrust={() => setTrustPrompt(true)}
            onRevokeTrust={() => setCourseTrust(false)}
            onReturnToDraft={trial ? () => void handleReturnToDraft() : undefined}
            activeChapterId={activeChapterId}
            focusMode={focusMode}
            pendingReload={pendingReload}
            reloading={reloadingCourse}
            messages={messages}
            onSelectChapter={setActiveChapterId}
            onToggleFocusMode={() => setFocusMode((prev) => !prev)}
            onReloadCourse={() => void handleReloadCourse()}
            onDismissReload={() => setPendingReload(null)}
            onCopyCommand={(text) => void handleCopyCommand(text)}
          />
        ) : (
          <WelcomeHome
            messages={messages}
            locale={settings.locale}
            builtinCourses={builtinCourses}
            recents={recents}
            onOpenRecentWorkspace={handleOpenRecentWorkspace}
            onOpenBuiltinCourse={(courseId) =>
              void handleOpenCourseRequest(
                { kind: "builtin", courseId },
                false
              )
            }
            onOpenImportModal={() => showImport("localMarkdown")}
            onOpenAuthorModal={() => showImport("author")}
            onPlaceholderAction={handlePlaceholderAction}
          />
        )}
      </main>

      {importMode && <ImportCourseModal
        initialMode={importMode}
        messages={messages}
        errorText={importError}
        submitting={importSubmitting}
        onClose={() => {
          setImportMode(null);
          setImportError(null);
        }}
        onBrowsePath={handleBrowsePath}
        onSubmit={(request) => handleOpenCourseRequest(request, true)}
        onOpenDraft={handleOpenDraft}
        onClearError={() => setImportError(null)}
      />}

      {navigation.pending && (
        <WorkspaceDialog title={messages.workspace.switchTitle} cancel={messages.workspace.cancel}
          busy={navigation.switching} onCancel={navigation.cancel}>
          <button type="button" className="btn btn--secondary btn--sm" disabled={navigation.switching}
            onClick={() => navigation.decide(false)}>{messages.workspace.discard}</button>
          <button type="button" className="btn btn--primary btn--sm" disabled={navigation.switching || (!authoring && !activeCourse?.trusted)}
            onClick={() => navigation.decide(true)}>{messages.workspace.saveAndSwitch}</button>
        </WorkspaceDialog>
      )}

      {trustPrompt && activeCourse && (
        <WorkspaceDialog
          title={activeCourse.source.kind === "gitPack" ? messages.coursePack.gitTrustTitle : messages.workspace.trustTitle}
          detail={<>{messages.workspace.trustDetail}<span className="mono workspace-dialog-path">{courseSourceLocation(activeCourse.source)}</span></>}
          cancel={messages.workspace.cancel}
          busy={navigation.switching}
          onCancel={() => setTrustPrompt(false)}
        >
          <button type="button" className="btn btn--primary btn--sm" disabled={navigation.switching}
            onClick={() => setCourseTrust(true)}>{messages.workspace.trust}</button>
        </WorkspaceDialog>
      )}

      <AppearanceDrawer
        open={appearanceOpen}
        settings={settings}
        messages={messages}
        onClose={handleCloseAppearance}
        onUpdateSettings={(patch) => void handleUpdateSettings(patch)}
        onCopyThemeConfig={() => void handleCopyThemeConfig()}
      />

      {runtimeSettingsOpen && (
        <RuntimeSettingsDialog {...runtimePanel} onClose={closeRuntimeSettings} />
      )}

      <ToastStack toasts={toasts} />
    </div>
  );
}
