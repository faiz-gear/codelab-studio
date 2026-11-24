import { useCallback, useEffect, useRef, useState } from "react";
import {
  fetchBootstrap,
  updateSettings,
  type CoreClient,
} from "./core/client";
import {
  CoreCommandError,
  DEFAULT_SETTINGS,
  type GlobalSettings,
  type RecentWorkspace,
  type UpdateSettingsPatch,
} from "./core/contracts";
import { AppearanceDrawer } from "./components/AppearanceDrawer";
import { ToastStack, type ToastMessage, type ToastTone } from "./components/ToastStack";
import { TopBar } from "./components/TopBar";
import { WelcomeHome } from "./components/WelcomeHome";
import { formatDomainError, getMessages, interpolate } from "./i18n";
import {
  applySettingsToDocument,
  buildThemeConfigSnippet,
} from "./theme/applyTheme";

export interface AppProps {
  client: CoreClient;
}

const TOAST_TTL_MS = 3800;

export function App({ client }: AppProps) {
  const [settings, setSettings] = useState<GlobalSettings>(DEFAULT_SETTINGS);
  const [recents, setRecents] = useState<RecentWorkspace[]>([]);
  const [appearanceOpen, setAppearanceOpen] = useState(false);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const appearanceTriggerRef = useRef<HTMLButtonElement | null>(null);
  const nextToastId = useRef(1);
  const localeRef = useRef(settings.locale);
  localeRef.current = settings.locale;

  const pushToast = useCallback((text: string, tone: ToastTone = "info") => {
    const id = nextToastId.current++;
    setToasts((prev) => [...prev.slice(-1), { id, tone, text }]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
    }, TOAST_TTL_MS);
  }, []);

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
          }
        });

        const bootstrap = await fetchBootstrap(client);
        if (!active) return;
        setSettings(bootstrap.settings);
        setRecents(bootstrap.recents);
        applySettingsToDocument(bootstrap.settings);

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
  }, [client, pushToast]);

  const messages = getMessages(settings.locale);

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
        appearanceOpen={appearanceOpen}
        appearanceTriggerRef={appearanceTriggerRef}
        onOpenAppearance={() => setAppearanceOpen(true)}
        onPlaceholderAction={handlePlaceholderAction}
      />

      <main className="view-host" id="mainView">
        <WelcomeHome
          messages={messages}
          locale={settings.locale}
          recents={recents}
          onPlaceholderAction={handlePlaceholderAction}
        />
      </main>

      <AppearanceDrawer
        open={appearanceOpen}
        settings={settings}
        messages={messages}
        onClose={handleCloseAppearance}
        onUpdateSettings={(patch) => void handleUpdateSettings(patch)}
        onCopyThemeConfig={() => void handleCopyThemeConfig()}
      />

      <ToastStack toasts={toasts} />
    </div>
  );
}
