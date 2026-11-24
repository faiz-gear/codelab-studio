import type { CoreClient, CoreEventListener, Unsubscribe } from "../core/client";
import {
  CoreCommandError,
  DEFAULT_SETTINGS,
  type AppCommand,
  type AppEvent,
  type AppResponse,
  type DomainError,
  type GlobalSettings,
  type RecentWorkspace,
  type StorageRecoveryNotice,
  type TrustEntry,
} from "../core/contracts";

export interface FakeCoreClientOptions {
  settings?: Partial<GlobalSettings>;
  recents?: RecentWorkspace[];
  trustEntries?: TrustEntry[];
  recoveryNotices?: StorageRecoveryNotice[];
}

export class FakeCoreClient implements CoreClient {
  private settings: GlobalSettings;
  private recents: RecentWorkspace[];
  private trustEntries: TrustEntry[];
  private recoveryNotices: StorageRecoveryNotice[];
  private shutDown = false;
  private readonly listeners = new Set<CoreEventListener>();
  private nextErrors = new Map<AppCommand["type"], DomainError>();
  readonly commandLog: AppCommand[] = [];

  constructor(options: FakeCoreClientOptions = {}) {
    this.settings = { ...DEFAULT_SETTINGS, ...options.settings };
    this.recents = options.recents ? [...options.recents] : [];
    this.trustEntries = options.trustEntries ? [...options.trustEntries] : [];
    this.recoveryNotices = options.recoveryNotices
      ? [...options.recoveryNotices]
      : [];
  }

  failNext(commandType: AppCommand["type"], error: DomainError): void {
    this.nextErrors.set(commandType, error);
  }

  emitEvent(event: AppEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  async dispatch(command: AppCommand): Promise<AppResponse> {
    this.commandLog.push(command);

    const injected = this.nextErrors.get(command.type);
    if (injected) {
      this.nextErrors.delete(command.type);
      throw new CoreCommandError(injected);
    }

    switch (command.type) {
      case "bootstrap":
        return {
          type: "bootstrap",
          payload: {
            settings: { ...this.settings },
            recents: [...this.recents],
            trustEntries: [...this.trustEntries],
            recoveryNotices: [...this.recoveryNotices],
          },
        };
      case "getSettings":
        return { type: "settings", payload: { ...this.settings } };
      case "updateSettings": {
        const patch = command.payload;
        if (
          patch.codeFontSize !== undefined &&
          (!Number.isFinite(patch.codeFontSize) ||
            patch.codeFontSize < 11 ||
            patch.codeFontSize > 20)
        ) {
          throw new CoreCommandError({
            code: "INVALID_CODE_FONT_SIZE",
            details: { min: "11", max: "20" },
          });
        }
        this.settings = {
          ...this.settings,
          ...(patch.theme !== undefined ? { theme: patch.theme } : {}),
          ...(patch.accent !== undefined ? { accent: patch.accent } : {}),
          ...(patch.codeFontSize !== undefined
            ? { codeFontSize: patch.codeFontSize }
            : {}),
          ...(patch.locale !== undefined ? { locale: patch.locale } : {}),
        };
        this.emitEvent({
          type: "settingsUpdated",
          payload: { ...this.settings },
        });
        return { type: "settings", payload: { ...this.settings } };
      }
      case "listRecents":
        return { type: "recents", payload: [...this.recents] };
      case "recordRecentWorkspace": {
        const input = command.payload;
        const entry: RecentWorkspace = {
          ...input,
          workspacePath: input.workspacePath.trim(),
          lastOpenedAt: new Date().toISOString(),
        };
        this.recents = [
          entry,
          ...this.recents.filter((r) => r.workspacePath !== entry.workspacePath),
        ].slice(0, 20);
        this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
        return { type: "recents", payload: [...this.recents] };
      }
      case "removeRecentWorkspace": {
        const target = command.payload.workspacePath.trim();
        this.recents = this.recents.filter((r) => r.workspacePath !== target);
        this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
        return { type: "recents", payload: [...this.recents] };
      }
      case "listTrustEntries":
        return { type: "trustEntries", payload: [...this.trustEntries] };
      case "checkTrust": {
        const normalized = command.payload.source.trim();
        const found = this.trustEntries.find(
          (entry) =>
            entry.sourceKind === command.payload.sourceKind &&
            entry.normalizedSource === normalized
        );
        return {
          type: "trustState",
          payload: {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trusted: Boolean(found),
            trustedAt: found?.trustedAt ?? null,
          },
        };
      }
      case "grantTrust": {
        const normalized = command.payload.source.trim();
        const trustedAt = new Date().toISOString();
        this.trustEntries = [
          {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trustedAt,
          },
          ...this.trustEntries.filter(
            (e) =>
              !(
                e.sourceKind === command.payload.sourceKind &&
                e.normalizedSource === normalized
              )
          ),
        ];
        this.emitEvent({
          type: "trustUpdated",
          payload: [...this.trustEntries],
        });
        return {
          type: "trustState",
          payload: {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trusted: true,
            trustedAt,
          },
        };
      }
      case "revokeTrust": {
        const normalized = command.payload.source.trim();
        this.trustEntries = this.trustEntries.filter(
          (e) =>
            !(
              e.sourceKind === command.payload.sourceKind &&
              e.normalizedSource === normalized
            )
        );
        this.emitEvent({
          type: "trustUpdated",
          payload: [...this.trustEntries],
        });
        return { type: "trustEntries", payload: [...this.trustEntries] };
      }
      case "shutdown": {
        const report = {
          alreadyShutDown: this.shutDown,
          hooksExecuted: 0,
        };
        if (!this.shutDown) {
          this.shutDown = true;
          this.emitEvent({ type: "shutdownCompleted", payload: report });
        }
        return { type: "shutdown", payload: report };
      }
    }
  }

  async subscribe(listener: CoreEventListener): Promise<Unsubscribe> {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}
