import type { CoreClient } from "../core/client";
import type { AppEvent, LanguageDiagnostic, LanguageDocument, LanguageFeature, LanguageStatusChanged } from "../core/contracts";

type LanguageEvent = Extract<AppEvent, { type: "languageStatusChanged" | "languageDiagnostics" }>;

const EMPTY = { status: null, diagnostics: [] } as const;

export class LanguageSession {
  private document: LanguageDocument | null = null;
  private version = 0;
  private openedAt = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private synced: Promise<void> | undefined;
  private state: { status: LanguageStatusChanged | null; diagnostics: readonly LanguageDiagnostic[] } = EMPTY;
  private listeners = new Set<() => void>();

  constructor(private client: CoreClient, private workspaceId: number) {}

  snapshot = () => this.state;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  private update(state: typeof this.state) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }

  select(document: LanguageDocument | null) {
    if (document?.path === this.document?.path && document?.text === this.document?.text) return;
    const switched = document?.path !== this.document?.path;
    this.document = document;
    this.version++;
    this.synced = undefined;
    clearTimeout(this.timer);
    if (switched) this.openedAt = this.version;
    this.update({ status: switched ? null : this.state.status, diagnostics: [] });
    if (switched) void this.sync();
    else this.timer = setTimeout(() => void this.sync(), 120);
  }

  suspend() {
    clearTimeout(this.timer);
    this.document = null;
    this.version++;
    this.synced = undefined;
    this.update(EMPTY);
  }

  handle(event: LanguageEvent) {
    const { payload } = event;
    if (payload.workspaceId !== this.workspaceId || payload.path !== this.document?.path ||
        payload.version < this.openedAt || payload.version > this.version) return;
    if (event.type === "languageStatusChanged") {
      if (payload.version < (this.state.status?.version ?? 0)) return;
      this.update({
        status: event.payload,
        diagnostics: event.payload.status === "ready" ? this.state.diagnostics : [],
      });
    } else if (payload.version === this.version && this.state.status?.status === "ready") {
      this.update({ status: this.state.status, diagnostics: event.payload.diagnostics });
    }
  }

  private sync() {
    clearTimeout(this.timer);
    if (this.synced) return this.synced;
    const version = this.version;
    const status = this.state.status;
    this.synced = this.client.dispatch({
      type: "syncLanguageDocument",
      payload: { workspaceId: this.workspaceId, version, document: this.document },
    }).then(() => {}, () => {
      if (version === this.version && status) {
        this.update({ status: { ...status, status: "degraded" }, diagnostics: [] });
      }
    });
    return this.synced;
  }

  async request(feature: LanguageFeature, path: string, line: number, character: number, cancelled: () => boolean) {
    if (path !== this.document?.path || this.state.status?.status !== "ready" || cancelled()) return null;
    const version = this.version;
    try {
      await this.sync();
      if (version !== this.version || cancelled()) return null;
      const response = await this.client.dispatch({
        type: "requestLanguageFeature",
        payload: { workspaceId: this.workspaceId, version, feature, line, character },
      });
      if (version === this.version && !cancelled() && response.type === "languageFeature") return response.payload;
    } catch {
      // The editor remains usable when an optional request fails.
    }
    return null;
  }
}
