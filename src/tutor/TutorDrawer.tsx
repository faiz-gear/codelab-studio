import { useEffect, useRef, useState } from "react";
import {
  CloseIcon,
  FileCodeIcon,
  RefreshIcon,
  SparklesIcon,
  StopSquareIcon,
  TrashIcon,
} from "../components/Icons";
import { expectResponse, type CoreClient } from "../core/client";
import {
  ACP_AGENT_PRESETS,
  type GlobalSettings,
  type LoadedCourse,
  type TeachingStyle,
  type TutorFileChange,
  type TutorSessionSnapshot,
  type UpdateSettingsPatch,
} from "../core/contracts";
import { formatDomainError, type Messages } from "../i18n";
import { domainError, type WorkspaceEditorSession } from "../workspace/editorSession";
import { TutorFileDiffDialog } from "./TutorFileDiffDialog";
import { TutorPermissionCard } from "./TutorPermissionCard";

export interface TutorDrawerProps {
  open: boolean;
  overlay: boolean;
  client?: CoreClient;
  course: LoadedCourse;
  session: WorkspaceEditorSession | null;
  settings: GlobalSettings;
  messages: Messages;
  snapshot: TutorSessionSnapshot | null;
  onSnapshotChange: (snapshot: TutorSessionSnapshot | null) => void;
  onUpdateSettings: (patch: UpdateSettingsPatch) => void;
  onOpenTutorSettings: (trigger: HTMLElement) => void;
  onRequestTrust: () => void;
  onChooseWorkspace: () => void;
  onClose: () => void;
  onError: (error: unknown) => void;
}

export function TutorDrawer({
  open,
  overlay,
  client,
  course,
  session,
  settings,
  messages,
  snapshot,
  onSnapshotChange,
  onUpdateSettings,
  onOpenTutorSettings,
  onRequestTrust,
  onChooseWorkspace,
  onClose,
  onError,
}: TutorDrawerProps) {
  const t = messages.tutor;
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [diffChange, setDiffChange] = useState<TutorFileChange | null>(null);
  const drawerRef = useRef<HTMLElement | null>(null);
  const streamRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLTextAreaElement | null>(null);

  const workspaceId = session?.workspace.id ?? null;

  useEffect(() => {
    if (!open || !client || !course.trusted || workspaceId === null) return;
    let active = true;
    void client
      .dispatch({ type: "getTutorSession", payload: { workspaceId } })
      .then((response) => {
        if (!active) return;
        onSnapshotChange(expectResponse(response, "tutorSession").payload);
      })
      .catch((error: unknown) => {
        if (active) onError(error);
      });
    return () => {
      active = false;
    };
  }, [client, course.trusted, onError, onSnapshotChange, open, workspaceId]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (document.querySelector("dialog[open]")) return;
      if (overlay || drawerRef.current?.contains(document.activeElement)) {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open, overlay]);

  const messageCount = snapshot?.messages.length ?? 0;
  const lastContentLength = snapshot?.messages.at(-1)?.content.length ?? 0;

  useEffect(() => {
    if (!open) return;
    if (streamRef.current) {
      streamRef.current.scrollTop = streamRef.current.scrollHeight;
    }
  }, [open, messageCount, lastContentLength]);

  useEffect(() => {
    if (open && course.trusted && workspaceId !== null) {
      queueMicrotask(() => inputRef.current?.focus());
    }
  }, [course.trusted, open, workspaceId]);

  if (!open) return null;

  const isStreaming =
    snapshot?.status === "starting" ||
    snapshot?.status === "streaming" ||
    snapshot?.status === "awaitingPermission";
  const activeStyle: TeachingStyle =
    snapshot?.teachingStyle ?? settings.tutor.teachingStyle;
  const presetLabel =
    ACP_AGENT_PRESETS.find((item) => item.id === settings.tutor.preset)?.label ??
    t.presets[settings.tutor.preset];
  const agentDisplay = snapshot?.agentName ?? presetLabel;

  const runAction = async (work: () => Promise<void>) => {
    setBusy(true);
    try {
      await work();
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };

  const handleSend = () => {
    if (!client || workspaceId === null || !course.trusted || isStreaming || busy) return;
    const prompt = draft.trim();
    if (!prompt) return;
    setDraft("");
    void runAction(async () => {
      const response = await client.dispatch({
        type: "sendTutorPrompt",
        payload: { workspaceId, prompt, teachingStyle: activeStyle },
      });
      onSnapshotChange(expectResponse(response, "tutorSession").payload);
    });
  };

  const handleStop = () => {
    if (!client || workspaceId === null) return;
    void runAction(async () => {
      const response = await client.dispatch({
        type: "stopTutorTurn",
        payload: { workspaceId },
      });
      onSnapshotChange(expectResponse(response, "tutorSession").payload);
    });
  };

  const handleRetry = () => {
    if (!client || workspaceId === null) return;
    void runAction(async () => {
      const response = await client.dispatch({
        type: "retryTutorTurn",
        payload: { workspaceId },
      });
      onSnapshotChange(expectResponse(response, "tutorSession").payload);
    });
  };

  const handleRespondPermission = (requestId: string, allow: boolean) => {
    if (!client || workspaceId === null) return;
    void runAction(async () => {
      const response = await client.dispatch({
        type: "respondTutorPermission",
        payload: { workspaceId, requestId, allow },
      });
      onSnapshotChange(expectResponse(response, "tutorSession").payload);
    });
  };

  const handleClear = () => {
    if (!client || workspaceId === null) return;
    void runAction(async () => {
      const response = await client.dispatch({
        type: "clearTutorSession",
        payload: { workspaceId },
      });
      onSnapshotChange(expectResponse(response, "tutorSession").payload);
    });
  };

  const handleStyleSwitch = (teachingStyle: TeachingStyle) => {
    onUpdateSettings({ tutor: { teachingStyle } });
  };

  return (
    <>
      <aside
        ref={drawerRef}
        id="tutor-drawer"
        className={`tutor-drawer${overlay ? " tutor-drawer--overlay" : ""}`}
        aria-label={t.drawerAriaLabel}
        data-testid="tutor-drawer"
        data-layout={overlay ? "overlay" : "docked"}
      >
        <header className="tutor-head">
          <div className="tutor-head-left">
            <span className="tutor-head-title">
              <SparklesIcon size={14} />
              <span>{t.drawerTitle}</span>
            </span>
            <button
              type="button"
              className="badge badge--ai tutor-agent-badge"
              title={t.configureAgent}
              onClick={(event) => onOpenTutorSettings(event.currentTarget)}
            >
              ACP · {agentDisplay}
            </button>
          </div>
          <div className="tutor-head-actions">
            {course.trusted && workspaceId !== null && (
              <button
                type="button"
                className="btn btn--ghost btn--icon btn--xs"
                aria-label={t.clearSession}
                title={t.clearSession}
                disabled={busy || (!snapshot?.messages.length && !snapshot?.processRunning)}
                onClick={handleClear}
              >
                <TrashIcon size={13} />
              </button>
            )}
            <button
              type="button"
              className="btn btn--ghost btn--icon btn--xs"
              aria-label={t.closeDrawer}
              title={t.closeDrawer}
              onClick={onClose}
            >
              <CloseIcon size={14} />
            </button>
          </div>
        </header>

        <div className="tutor-toolbar">
          <div
            className="segmented tutor-style-toggle"
            role="group"
            aria-label={t.styleGroupLabel}
          >
            {(["socratic", "direct"] as const).map((style) => (
              <button
                key={style}
                type="button"
                className={`seg-btn${activeStyle === style ? " is-active" : ""}`}
                aria-pressed={activeStyle === style}
                onClick={() => handleStyleSwitch(style)}
              >
                {t.styles[style]}
              </button>
            ))}
          </div>
          <span className="badge badge--neutral tutor-policy-pill">
            {settings.tutor.permissionPolicy === "strict_ask_all"
              ? t.policies.strict_ask_all.title
              : t.policies.guided_readonly.title}
          </span>
        </div>

        {!course.trusted ? (
          <div className="tutor-empty" data-testid="tutor-untrusted-notice">
            <SparklesIcon size={20} />
            <h3>{t.untrustedTitle}</h3>
            <p>{t.untrustedHint}</p>
            <button
              type="button"
              className="btn btn--primary btn--sm"
              onClick={onRequestTrust}
            >
              {messages.workspace.trust}
            </button>
          </div>
        ) : workspaceId === null ? (
          <div className="tutor-empty" data-testid="tutor-no-workspace-notice">
            <SparklesIcon size={20} />
            <h3>{t.noWorkspaceTitle}</h3>
            <p>{t.noWorkspaceHint}</p>
            <button
              type="button"
              className="btn btn--primary btn--sm"
              onClick={onChooseWorkspace}
            >
              {messages.workspace.choose}
            </button>
          </div>
        ) : (
          <>
            <div
              ref={streamRef}
              className="tutor-stream"
              role="log"
              aria-live="polite"
              data-testid="tutor-stream"
            >
              {(!snapshot || snapshot.messages.length === 0) && (
                <div className="tutor-empty">
                  <span className="badge badge--neutral">{t.lazyIdleBadge}</span>
                  <h3>{t.emptyTitle}</h3>
                  <p>{t.emptyHint}</p>
                </div>
              )}

              {snapshot?.messages.map((message) => (
                <article
                  key={message.id}
                  className={`tutor-msg tutor-msg--${message.role}`}
                  data-testid={`tutor-msg-${message.role}`}
                >
                  <div className="tutor-msg-meta">
                    <span className="tutor-msg-style">
                      {t.styles[message.teachingStyle]}
                    </span>
                    {message.role === "assistant" && message.status === "cancelled" && (
                      <span className="badge badge--neutral">{t.cancelled}</span>
                    )}
                  </div>

                  {message.permissions.length > 0 && (
                    <div className="tutor-msg-perms">
                      {message.permissions.map((perm) => (
                        <TutorPermissionCard
                          key={perm.id}
                          permission={perm}
                          messages={messages}
                          busy={busy}
                          onRespond={handleRespondPermission}
                        />
                      ))}
                    </div>
                  )}

                  {message.content && (
                    <div className="tutor-msg-body">{message.content}</div>
                  )}

                  {message.role === "assistant" &&
                    message.status === "streaming" &&
                    !message.content && (
                      <p className="tutor-msg-pending">
                        {snapshot.status === "starting" ? t.starting : t.streaming}
                      </p>
                    )}

                  {message.fileChanges.length > 0 && (
                    <div className="tutor-file-changes">
                      {message.fileChanges.map((change) => (
                        <div key={change.path} className="tutor-file-change">
                          <span className="tutor-file-change-path mono">
                            <FileCodeIcon size={12} />
                            <span>{change.path}</span>
                          </span>
                          <button
                            type="button"
                            className="btn btn--secondary btn--xs btn--pill"
                            onClick={() => setDiffChange(change)}
                          >
                            {t.viewDiff}
                          </button>
                        </div>
                      ))}
                    </div>
                  )}

                  {message.status === "failed" && message.error && (
                    <div className="inline-alert inline-alert--danger" role="alert">
                      {formatDomainError(message.error, settings.locale)}
                    </div>
                  )}
                </article>
              ))}

              {snapshot?.canRetry && (
                <div className="tutor-retry-bar">
                  <button
                    type="button"
                    className="btn btn--secondary btn--sm"
                    disabled={busy}
                    onClick={handleRetry}
                  >
                    <RefreshIcon size={13} />
                    <span>{t.retry}</span>
                  </button>
                </div>
              )}
            </div>

            <footer className="tutor-composer-wrap">
              <div className="tutor-composer">
                <textarea
                  ref={inputRef}
                  aria-label={t.composerLabel}
                  placeholder={t.composerPlaceholder}
                  rows={2}
                  value={draft}
                  disabled={busy}
                  onChange={(event) => setDraft(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                      event.preventDefault();
                      handleSend();
                    }
                  }}
                />
                <div className="tutor-composer-foot">
                  <span className="tutor-status-meta mono">
                    {snapshot?.sessionReused
                      ? t.reusedBadge
                      : snapshot?.processRunning
                        ? `ACP · ${agentDisplay}`
                        : t.lazyIdleBadge}
                  </span>
                  {isStreaming ? (
                    <button
                      type="button"
                      className="btn btn--danger btn--xs btn--pill"
                      disabled={busy}
                      onClick={handleStop}
                    >
                      <StopSquareIcon size={11} />
                      <span>{t.stop}</span>
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="btn btn--primary btn--xs btn--pill"
                      disabled={busy || !draft.trim()}
                      onClick={handleSend}
                    >
                      {t.send}
                    </button>
                  )}
                </div>
              </div>
            </footer>
          </>
        )}
      </aside>

      {diffChange && (
        <TutorFileDiffDialog
          change={diffChange}
          settings={settings}
          messages={messages}
          onClose={() => setDiffChange(null)}
          onOpenInEditor={
            session
              ? (filePath) => {
                  void session.open(filePath).catch((err) => onError(domainError(err)));
                }
              : undefined
          }
        />
      )}
    </>
  );
}
