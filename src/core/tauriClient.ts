import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CoreClient, CoreEventListener, Unsubscribe } from "./client";
import {
  CoreCommandError,
  isDomainError,
  type AppCommand,
  type AppEvent,
  type AppResponse,
} from "./contracts";

export const CORE_EVENT_CHANNEL = "codelab://core-event";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export class TauriCoreClient implements CoreClient {
  async dispatch(command: AppCommand): Promise<AppResponse> {
    if (!isTauriRuntime()) {
      throw new CoreCommandError({ code: "IPC_UNAVAILABLE" });
    }
    try {
      return await invoke<AppResponse>("dispatch_command", { command });
    } catch (err) {
      if (isDomainError(err)) {
        throw new CoreCommandError(err);
      }
      if (typeof err === "string") {
        try {
          const parsed: unknown = JSON.parse(err);
          if (isDomainError(parsed)) {
            throw new CoreCommandError(parsed);
          }
        } catch {
          // Non-JSON transport failure falls through to UNEXPECTED_ERROR.
        }
      }
      throw new CoreCommandError({
        code: "UNEXPECTED_ERROR",
        details: { reason: String(err) },
      });
    }
  }

  async subscribe(listener: CoreEventListener): Promise<Unsubscribe> {
    if (!isTauriRuntime()) {
      return () => {};
    }
    const unlisten = await listen<AppEvent>(CORE_EVENT_CHANNEL, (event) => {
      listener(event.payload);
    });
    return () => {
      unlisten();
    };
  }
}
