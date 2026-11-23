import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  FsDeltaEvent,
  LspDiagnosticsEvent,
  LspStatusEvent,
  PortConflictEvent,
  PtyStateEvent,
  SessionEpoch,
  SessionEventEnvelope,
  StorageCorruptedEvent,
} from "./generated";

export type { UnlistenFn };

export interface SessionEventMap {
  "workspace://fs-delta": FsDeltaEvent;
  "workspace://storage-corrupted": StorageCorruptedEvent;
  "pty://state": PtyStateEvent;
  "pty://port-conflict": PortConflictEvent;
  "lsp://status": LspStatusEvent;
  "lsp://diagnostics": LspDiagnosticsEvent;
}

export type SessionEventName = keyof SessionEventMap;

export type SessionEventListenFn<T> = (
  eventName: string,
  callback: (event: Pick<Event<SessionEventEnvelope<T>>, "payload">) => void,
) => Promise<UnlistenFn>;

function defaultListen<T>(
  eventName: string,
  callback: (event: Pick<Event<SessionEventEnvelope<T>>, "payload">) => void,
): Promise<UnlistenFn> {
  return listen<SessionEventEnvelope<T>>(eventName, callback);
}

export async function subscribeSessionEvent<T>(
  eventName: string,
  getActiveEpoch: () => SessionEpoch,
  handler: (payload: T) => void,
  listenFn: SessionEventListenFn<T> = defaultListen,
): Promise<UnlistenFn> {
  return listenFn(eventName, (event) => {
    const envelope = event.payload;
    if (envelope.epoch !== getActiveEpoch()) {
      return;
    }
    handler(envelope.payload);
  });
}
