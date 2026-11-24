import { useCallback, useEffect, useRef, useState } from "react";
import {
  closeActiveCourse,
  dispatchPreflight,
  fetchBootstrap,
  openCourse,
  reloadActiveCourse,
  updateSettings,
  type CoreClient,
} from "./core/client";
import {
  CoreCommandError,
  courseSourceKey,
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
} from "./core/contracts";
import { AppearanceDrawer } from "./components/AppearanceDrawer";
import { CourseWorkbench } from "./components/CourseWorkbench";
import { ImportCourseModal } from "./components/ImportCourseModal";
import { RuntimeSettingsDialog } from "./components/RuntimeSettingsDialog";
import {
  ToastStack,
  type ToastMessage,
  type ToastTone,
} from "./components/ToastStack";
import { TopBar } from "./components/TopBar";
import { WelcomeHome } from "./components/WelcomeHome";
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
  const [activeChapterId, setActiveChapterId] = useState<string>("");
  const [focusMode, setFocusMode] = useState(false);
  const [pendingReload, setPendingReload] =
    useState<CourseSourceChangedEvent | null>(null);
  const [reloadingCourse, setReloadingCourse] = useState(false);

  const [appearanceOpen, setAppearanceOpen] = useState(false);
  const [runtimeSettingsOpen, setRuntimeSettingsOpen] = useState(false);
  const [preflight, setPreflight] = useState<PreflightReport | null>(null);
  const [preflightError, setPreflightError] = useState<DomainError | null>(null);
  const [importOpen, setImportOpen] = useState(false);
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

  useEffect(() => {
    activeCourseRef.current = activeCourse;
  }, [activeCourse]);

  const pushToast = useCallback((text: string, tone: ToastTone = "info") => {
    const id = nextToastId.current++;
    setToasts((prev) => [...prev.slice(-1), { id, tone, text }]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
    }, TOAST_TTL_MS);
  }, []);

  const applyLoadedCourse = useCallback((loaded: LoadedCourse) => {
    const prevCourse = activeCourseRef.current;
    const isSameCourse = prevCourse?.courseId === loaded.courseId;

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

    if (!loaded.isBuiltin) {
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
    if (!activeCourse) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (importOpen || appearanceOpen || runtimeSettingsOpen) return;
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
  }, [activeCourse, importOpen, appearanceOpen, runtimeSettingsOpen]);

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

  const handleOpenCourseRequest = useCallback(
    async (request: OpenCourseRequest, fromImportModal = false) => {
      if (fromImportModal) {
        setImportSubmitting(true);
        setImportError(null);
      }
      try {
        const loaded = await openCourse(client, request);
        applyLoadedCourse(loaded);
        setFocusMode(false);
        if (fromImportModal) {
          setImportOpen(false);
        }
      } catch (err) {
        if (err instanceof CoreCommandError) {
          const text = formatDomainError(err.domainError, localeRef.current);
          if (fromImportModal) {
            setImportError(text);
          } else {
            pushToast(text, "err");
          }
        }
      } finally {
        if (fromImportModal) {
          setImportSubmitting(false);
        }
      }
    },
    [applyLoadedCourse, client, pushToast]
  );

  const handleSelectCourseSummary = useCallback(
    (summary: CourseSummary) => {
      void handleOpenCourseRequest(openRequestFromSource(summary.source), false);
    },
    [handleOpenCourseRequest]
  );

  const handleGoHome = useCallback(() => {
    if (!activeCourseRef.current) {
      window.scrollTo({ top: 0 });
      return;
    }
    setActiveCourse(null);
    setActiveChapterId("");
    setPendingReload(null);
    setFocusMode(false);
    void closeActiveCourse(client).catch(() => {
      // Closing an active course on return to Home is best-effort.
    });
  }, [client]);

  const handleReloadCourse = useCallback(async () => {
    setReloadingCourse(true);
    try {
      const reloaded = await reloadActiveCourse(client);
      applyLoadedCourse(reloaded);
      pushToast(
        interpolate(messages.workbench.reloadedToast, {
          title: reloaded.title,
        }),
        "ok"
      );
    } catch (err) {
      if (err instanceof CoreCommandError) {
        pushToast(formatDomainError(err.domainError, localeRef.current), "err");
      }
    } finally {
      setReloadingCourse(false);
    }
  }, [applyLoadedCourse, client, messages.workbench.reloadedToast, pushToast]);

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
        activeCourse={activeCourse}
        focusMode={focusMode}
        appearanceOpen={appearanceOpen}
        appearanceTriggerRef={appearanceTriggerRef}
        onGoHome={handleGoHome}
        onSelectCourse={handleSelectCourseSummary}
        onOpenImportModal={() => {
          setImportError(null);
          setImportOpen(true);
        }}
        onToggleFocusMode={() => setFocusMode((prev) => !prev)}
        onOpenAppearance={() => setAppearanceOpen(true)}
        runtimePanel={runtimePanel}
        runtimeSettingsOpen={runtimeSettingsOpen}
        onOpenRuntimeSettings={openRuntimeSettings}
      />

      <main className="view-host" id="mainView">
        {activeCourse ? (
          <CourseWorkbench
            course={activeCourse}
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
            onOpenBuiltinCourse={(courseId) =>
              void handleOpenCourseRequest(
                { kind: "builtin", courseId },
                false
              )
            }
            onOpenImportModal={() => {
              setImportError(null);
              setImportOpen(true);
            }}
            onPlaceholderAction={handlePlaceholderAction}
          />
        )}
      </main>

      <ImportCourseModal
        open={importOpen}
        messages={messages}
        errorText={importError}
        submitting={importSubmitting}
        onClose={() => {
          setImportOpen(false);
          setImportError(null);
        }}
        onBrowsePath={handleBrowsePath}
        onSubmit={(request) => handleOpenCourseRequest(request, true)}
        onClearError={() => setImportError(null)}
      />

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
