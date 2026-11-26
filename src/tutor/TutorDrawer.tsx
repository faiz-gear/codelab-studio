import { useEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import {
  CloseIcon,
  FileCodeIcon,
  RefreshIcon,
  SparklesIcon,
  StopSquareIcon,
  TrashIcon,
} from "../components/Icons";
import { trapFocusOnTab } from "../components/focusTrap";
import { expectResponse, type CoreClient } from "../core/client";
import {
  ACP_AGENT_PRESETS,
  MAX_CONTEXT_INPUT_TOKENS,
  type ContextChip,
  type ContextDimension,
  type ContextQuote,
  type ContextSnapshot,
  type ContextSnapshotInput,
  type GlobalSettings,
  type LearningProgress,
  type LoadedCourse,
  type TeachingStyle,
  type TutorFileChange,
  type TutorSessionSnapshot,
  type UpdateSettingsPatch,
} from "../core/contracts";
import { formatDomainError, interpolate, type Messages } from "../i18n";
import {
  domainError,
  editorText,
  type EditorTab,
  type WorkspaceEditorSession,
} from "../workspace/editorSession";
import { TutorFileDiffDialog } from "./TutorFileDiffDialog";
import { TutorPermissionCard } from "./TutorPermissionCard";
import type { EditorCursorContext, TutorAskRequest } from "./contextRequest";

const EMPTY_EDITOR_STATE = {
  tabs: [] as EditorTab[],
  activePath: null as string | null,
};
const STATIC_SUBSCRIBE = () => () => {};
const STATIC_EDITOR_SNAPSHOT = () => EMPTY_EDITOR_STATE;

function formatQuoteLocation(quote: ContextQuote): string | null {
  if (quote.filePath) {
    if (quote.startLine && quote.endLine && quote.endLine !== quote.startLine) {
      return `${quote.filePath}:L${quote.startLine}-${quote.endLine}`;
    }
    if (quote.startLine) {
      return `${quote.filePath}:L${quote.startLine}`;
    }
    return quote.filePath;
  }
  return quote.label ?? null;
}

function formatChipAccessibleName(chip: ContextChip, messages: Messages): string {
  const tc = messages.tutor.context;
  switch (chip.dimension) {
    case "chapter":
      return `${tc.dimensions.chapter} · ${chip.label}`;
    case "activeFile":
      return chip.detail ? `${chip.label} (${chip.detail})` : chip.label;
    case "terminalErrors":
      return interpolate(tc.terminalErrorsChip, { count: chip.label });
    case "verification":
      return interpolate(tc.verificationChip, { summary: chip.label });
  }
}

function renderChipLabel(chip: ContextChip, messages: Messages): ReactNode {
  const tc = messages.tutor.context;
  switch (chip.dimension) {
    case "chapter":
      return (
        <span className="ctx-chip-label">
          {tc.dimensions.chapter} · {chip.label}
        </span>
      );
    case "activeFile":
      return (
        <span className="ctx-chip-label">
          <span className="mono">{chip.label}</span>
          {chip.detail && <span className="mono ctx-chip-meta"> {chip.detail}</span>}
        </span>
      );
    case "terminalErrors":
      return (
        <span className="ctx-chip-label">
          {tc.dimensions.terminalErrors} <span className="mono">{chip.label}</span>
        </span>
      );
    case "verification":
      return (
        <span className="ctx-chip-label">
          {tc.dimensions.verification} <span className="mono">{chip.label}</span>
        </span>
      );
  }
}

export interface TutorDrawerProps {
  open: boolean;
  overlay: boolean;
  client?: CoreClient;
  course: LoadedCourse;
  activeChapterId?: string;
  session: WorkspaceEditorSession | null;
  editorCursor?: EditorCursorContext;
  verificationProgress?: LearningProgress | null;
  terminalRevision?: number;
  askRequest?: TutorAskRequest | null;
  onConsumeAskRequest?: (requestId: number) => void;
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
  activeChapterId = "",
  session,
  editorCursor,
  verificationProgress = null,
  terminalRevision = 0,
  askRequest = null,
  onConsumeAskRequest,
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
  const tc = t.context;
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [diffChange, setDiffChange] = useState<TutorFileChange | null>(null);
  const [pinnedQuote, setPinnedQuote] = useState<ContextQuote | null>(null);
  const [excludedDimensions, setExcludedDimensions] = useState<ContextDimension[]>([]);
  const [previewDimension, setPreviewDimension] = useState<ContextDimension | null>(null);
  const [contextState, setContextState] = useState<{
    snapshot: ContextSnapshot;
    quoteRef: ContextQuote | null;
  } | null>(null);
  const drawerRef = useRef<HTMLElement | null>(null);
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);
  const streamRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLTextAreaElement | null>(null);

  const editorState = useSyncExternalStore(
    session ? session.subscribe : STATIC_SUBSCRIBE,
    session ? session.snapshot : STATIC_EDITOR_SNAPSHOT
  );

  const workspaceId = session?.workspace.id ?? null;
  const activeTab = editorState.tabs.find((tab) => tab.file.path === editorState.activePath) ?? null;
  const activeFilePath = activeTab?.file.path ?? null;
  const activeFileText = activeTab ? editorText(activeTab) : null;
  const cursorStartLine =
    editorCursor?.filePath === activeFilePath ? editorCursor?.startLine ?? null : null;
  const cursorEndLine =
    editorCursor?.filePath === activeFilePath ? editorCursor?.endLine ?? null : null;

  const activeStyle: TeachingStyle =
    snapshot?.teachingStyle ?? settings.tutor.teachingStyle;

  const [lastWorkspaceId, setLastWorkspaceId] = useState(workspaceId);
  if (workspaceId !== lastWorkspaceId) {
    setLastWorkspaceId(workspaceId);
    setPinnedQuote(null);
    setExcludedDimensions([]);
    setPreviewDimension(null);
    setContextState(null);
  }

  const askId = askRequest?.id ?? 0;
  const [lastAskId, setLastAskId] = useState(0);
  if (askRequest && askId !== lastAskId) {
    setLastAskId(askId);
    setPinnedQuote(askRequest.quote);
    setDraft(askRequest.initialPrompt);
    setExcludedDimensions([]);
    setPreviewDimension(null);
  }

  useEffect(() => {
    if (askRequest && askId === lastAskId) {
      onConsumeAskRequest?.(askId);
    }
  }, [askId, askRequest, lastAskId, onConsumeAskRequest]);

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

  const contextInput: ContextSnapshotInput | null = useMemo(() => {
    if (workspaceId === null) return null;
    return {
      workspaceId,
      chapterId: activeChapterId || null,
      activeFilePath,
      activeFileText,
      activeFileStartLine: cursorStartLine,
      activeFileEndLine: cursorEndLine,
      quote: pinnedQuote,
      excludedDimensions,
      prompt: draft,
      teachingStyle: activeStyle,
    };
  }, [
    activeChapterId,
    activeFilePath,
    activeFileText,
    activeStyle,
    cursorEndLine,
    cursorStartLine,
    draft,
    excludedDimensions,
    pinnedQuote,
    workspaceId,
  ]);

  const activeChapterProgress =
    verificationProgress && activeChapterId
      ? verificationProgress.chapters[activeChapterId] ?? null
      : null;

  useEffect(() => {
    if (!open || !client || !course.trusted || !contextInput) return;
    let active = true;
    const requestedQuote = pinnedQuote;
    void client
      .dispatch({ type: "buildTutorContext", payload: contextInput })
      .then((response) => {
        if (!active) return;
        setContextState({
          snapshot: expectResponse(response, "tutorContext").payload,
          quoteRef: requestedQuote,
        });
      })
      .catch((error: unknown) => {
        if (active) onError(error);
      });
    return () => {
      active = false;
    };
  }, [
    activeChapterProgress,
    client,
    contextInput,
    course.trusted,
    onError,
    open,
    pinnedQuote,
    terminalRevision,
  ]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;
      if (event.key === "Tab" && overlay) {
        trapFocusOnTab(drawerRef.current, event);
        return;
      }
      if (event.key !== "Escape") return;
      if (document.querySelector('[data-testid="selection-ask-pill"]')) return;
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
    if (!open) return;
    if (course.trusted && workspaceId !== null) {
      queueMicrotask(() => inputRef.current?.focus());
    } else {
      queueMicrotask(() => closeButtonRef.current?.focus());
    }
  }, [askId, course.trusted, open, workspaceId]);

  if (!open) return null;

  const isStreaming =
    snapshot?.status === "starting" ||
    snapshot?.status === "streaming" ||
    snapshot?.status === "awaitingPermission";
  const presetLabel =
    ACP_AGENT_PRESETS.find((item) => item.id === settings.tutor.preset)?.label ??
    t.presets[settings.tutor.preset];
  const agentDisplay = snapshot?.agentName ?? presetLabel;

  const contextSnapshot = contextState?.snapshot ?? null;
  const displayedQuote =
    contextState && contextState.quoteRef === pinnedQuote
      ? contextState.snapshot.quote
      : pinnedQuote;
  const chips = contextSnapshot?.chips ?? [];
  const previewChip = previewDimension
    ? chips.find((chip) => chip.dimension === previewDimension) ?? null
    : null;
  const previewVerificationErrors =
    previewChip?.dimension === "verification"
      ? (activeChapterProgress?.lastCheckResults ?? [])
          .filter((result) => result.error !== null)
          .map((result) => ({
            checkId: result.checkId,
            text: formatDomainError(result.error!, settings.locale),
          }))
      : [];
  const estimatedTokens = contextSnapshot?.estimatedTokens ?? 0;
  const maxTokens = contextSnapshot?.maxTokens ?? MAX_CONTEXT_INPUT_TOKENS;

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
    if (!client || workspaceId === null || !course.trusted || isStreaming || busy || !contextInput) {
      return;
    }
    const prompt = draft.trim();
    if (!prompt) return;
    const payloadContext: ContextSnapshotInput = {
      ...contextInput,
      quote: displayedQuote,
      prompt,
    };
    setDraft("");
    setPinnedQuote(null);
    setPreviewDimension(null);
    setExcludedDimensions([]);
    void runAction(async () => {
      const response = await client.dispatch({
        type: "sendTutorPrompt",
        payload: {
          workspaceId,
          prompt,
          teachingStyle: activeStyle,
          context: payloadContext,
        },
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

  const handleRemoveChip = (dimension: ContextDimension) => {
    setExcludedDimensions((prev) =>
      prev.includes(dimension) ? prev : [...prev, dimension]
    );
    if (previewDimension === dimension) {
      setPreviewDimension(null);
    }
  };

  const quoteLocation = displayedQuote ? formatQuoteLocation(displayedQuote) : null;

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
              ref={closeButtonRef}
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

              {snapshot?.messages.map((message) => {
                const msgQuoteLoc = message.quote ? formatQuoteLocation(message.quote) : null;
                return (
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

                    {message.quote && (
                      <div className="tutor-msg-quote">
                        <div className="tutor-quote-meta">
                          <span className="badge badge--ai">
                            {tc.sources[message.quote.source]}
                          </span>
                          {msgQuoteLoc && <span className="mono">{msgQuoteLoc}</span>}
                        </div>
                        <pre className="tutor-quote-text mono">{message.quote.text}</pre>
                      </div>
                    )}

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
                );
              })}

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
              {displayedQuote && (
                <div
                  className="tutor-quote-card"
                  role="region"
                  aria-label={tc.quoteCardLabel}
                  data-testid="tutor-quote-card"
                  data-source={displayedQuote.source}
                >
                  <div className="tutor-quote-head">
                    <div className="tutor-quote-meta">
                      <span className="badge badge--ai">
                        {tc.sources[displayedQuote.source]}
                      </span>
                      {quoteLocation && (
                        <span className="tutor-quote-loc mono" title={quoteLocation}>
                          {quoteLocation}
                        </span>
                      )}
                    </div>
                    <button
                      type="button"
                      className="btn btn--ghost btn--icon btn--xs"
                      aria-label={tc.dismissQuote}
                      title={tc.dismissQuote}
                      onClick={() => setPinnedQuote(null)}
                    >
                      <CloseIcon size={12} />
                    </button>
                  </div>
                  <pre className="tutor-quote-text mono">{displayedQuote.text}</pre>
                </div>
              )}

              {previewChip && (
                <div
                  className="tutor-chip-preview"
                  role="region"
                  aria-label={formatChipAccessibleName(previewChip, messages)}
                  data-testid="context-chip-preview"
                >
                  <div className="tutor-chip-preview-head">
                    <span className="tutor-chip-preview-title">
                      {renderChipLabel(previewChip, messages)}
                    </span>
                    <span className="mono tutor-chip-preview-tok">
                      ~{previewChip.estimatedTokens} tok
                    </span>
                    <button
                      type="button"
                      className="btn btn--ghost btn--icon btn--xs"
                      aria-label={tc.closePreview}
                      onClick={() => setPreviewDimension(null)}
                    >
                      <CloseIcon size={12} />
                    </button>
                  </div>
                  {previewChip.truncated && (
                    <p className="tutor-chip-preview-note">{tc.trimmedChipHint}</p>
                  )}
                  {previewVerificationErrors.length > 0 && (
                    <div className="tutor-chip-preview-errors">
                      {previewVerificationErrors.map((err) => (
                        <p key={err.checkId} className="tutor-chip-preview-note">
                          {err.text}
                        </p>
                      ))}
                    </div>
                  )}
                  <pre className="tutor-chip-preview-body mono">
                    {previewChip.preview || tc.trimmedEmptyPreview}
                  </pre>
                </div>
              )}

              {(chips.length > 0 ||
                excludedDimensions.length > 0 ||
                contextSnapshot?.sensitiveFiltered ||
                contextSnapshot?.truncated) && (
                <div
                  className="tutor-context-bar"
                  role="region"
                  aria-label={tc.chipsGroupLabel}
                  data-testid="tutor-context-bar"
                >
                  {chips.map((chip) => {
                    const chipName = formatChipAccessibleName(chip, messages);
                    const isPreviewing = previewDimension === chip.dimension;
                    return (
                      <span
                        key={chip.dimension}
                        className={`ctx-chip${isPreviewing ? " is-previewing" : ""}`}
                        data-testid={`context-chip-${chip.dimension}`}
                        data-truncated={chip.truncated ? "true" : undefined}
                      >
                        <button
                          type="button"
                          className="ctx-chip-btn"
                          aria-pressed={isPreviewing}
                          aria-label={interpolate(tc.previewChip, { name: chipName })}
                          title={chipName}
                          onClick={() =>
                            setPreviewDimension((current) =>
                              current === chip.dimension ? null : chip.dimension
                            )
                          }
                        >
                          {renderChipLabel(chip, messages)}
                        </button>
                        <button
                          type="button"
                          className="ctx-chip-rm"
                          aria-label={interpolate(tc.removeChip, { name: chipName })}
                          onClick={() => handleRemoveChip(chip.dimension)}
                        >
                          ×
                        </button>
                      </span>
                    );
                  })}
                  {excludedDimensions.length > 0 && (
                    <button
                      type="button"
                      className="btn btn--ghost btn--xs ctx-restore-btn"
                      data-testid="context-restore-chips"
                      onClick={() => setExcludedDimensions([])}
                    >
                      {tc.restoreAll}
                    </button>
                  )}
                  {contextSnapshot?.truncated && (
                    <span
                      className="badge badge--warning tutor-ctx-flag"
                      data-testid="context-trimmed-badge"
                    >
                      {tc.trimmedBadge}
                    </span>
                  )}
                  {contextSnapshot?.sensitiveFiltered && (
                    <span
                      className="badge badge--neutral tutor-ctx-flag"
                      data-testid="context-sensitive-badge"
                    >
                      {tc.sensitiveFilteredBadge}
                    </span>
                  )}
                </div>
              )}

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
                    if (
                      event.key === "Enter" &&
                      !event.shiftKey &&
                      !event.nativeEvent.isComposing
                    ) {
                      event.preventDefault();
                      handleSend();
                    }
                  }}
                />
                <div className="tutor-composer-foot">
                  <div className="tutor-composer-meta">
                    <span className="tutor-status-meta mono">
                      {snapshot?.sessionReused
                        ? t.reusedBadge
                        : snapshot?.processRunning
                          ? `ACP · ${agentDisplay}`
                          : t.lazyIdleBadge}
                    </span>
                    <span
                      className="tutor-token-est mono"
                      data-testid="tutor-token-estimate"
                    >
                      {interpolate(tc.tokenBudget, {
                        used: estimatedTokens,
                        max: maxTokens,
                      })}
                    </span>
                  </div>
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
