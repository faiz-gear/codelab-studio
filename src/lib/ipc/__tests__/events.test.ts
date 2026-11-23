import { describe, expect, it, vi } from "vitest";
import { subscribeSessionEvent, type SessionEventListenFn } from "../events";
import type { FsDeltaEvent, SessionEpoch, SessionEventEnvelope } from "../generated";

function createFakeEventBus<T>() {
  let registeredName: string | null = null;
  let listener: ((event: { payload: SessionEventEnvelope<T> }) => void) | null = null;
  const unlisten = vi.fn(() => {
    listener = null;
  });

  const listenFn: SessionEventListenFn<T> = async (eventName, callback) => {
    registeredName = eventName;
    listener = callback;
    return unlisten;
  };

  const emit = (envelope: SessionEventEnvelope<T>) => {
    listener?.({ payload: envelope });
  };

  return {
    listenFn,
    emit,
    unlisten,
    getRegisteredName: () => registeredName,
  };
}

describe("subscribeSessionEvent", () => {
  it("drops stale events where epoch is less than activeEpoch", async () => {
    const bus = createFakeEventBus<FsDeltaEvent>();
    const received: FsDeltaEvent[] = [];

    await subscribeSessionEvent<FsDeltaEvent>(
      "workspace://fs-delta",
      () => 3,
      (payload) => received.push(payload),
      bus.listenFn,
    );

    expect(bus.getRegisteredName()).toBe("workspace://fs-delta");

    bus.emit({
      epoch: 1,
      payload: { deltas: [{ kind: "created", path: "main.go" }] },
    });
    bus.emit({
      epoch: 2,
      payload: { deltas: [{ kind: "removed", path: "old.go" }] },
    });

    expect(received).toEqual([]);
  });

  it("drops future events where epoch is greater than activeEpoch", async () => {
    const bus = createFakeEventBus<FsDeltaEvent>();
    const received: FsDeltaEvent[] = [];

    await subscribeSessionEvent<FsDeltaEvent>(
      "workspace://fs-delta",
      () => 2,
      (payload) => received.push(payload),
      bus.listenFn,
    );

    bus.emit({
      epoch: 3,
      payload: { deltas: [{ kind: "created", path: "future.go" }] },
    });

    expect(received).toEqual([]);
  });

  it("delivers payloads when epoch matches activeEpoch and respects epoch transitions", async () => {
    const bus = createFakeEventBus<FsDeltaEvent>();
    const received: FsDeltaEvent[] = [];
    let activeEpoch: SessionEpoch = 0;

    await subscribeSessionEvent<FsDeltaEvent>(
      "workspace://fs-delta",
      () => activeEpoch,
      (payload) => received.push(payload),
      bus.listenFn,
    );

    const firstPayload: FsDeltaEvent = {
      deltas: [{ kind: "created", path: "task-api/main.go" }],
    };
    bus.emit({ epoch: 0, payload: firstPayload });

    activeEpoch = 1;
    bus.emit({
      epoch: 0,
      payload: { deltas: [{ kind: "removed", path: "stale.go" }] },
    });

    const secondPayload: FsDeltaEvent = {
      deltas: [{ kind: "manualChanged", path: "README.md" }],
    };
    bus.emit({ epoch: 1, payload: secondPayload });

    expect(received).toEqual([firstPayload, secondPayload]);
  });

  it("invokes underlying unlisten cleanup when unsubscribed", async () => {
    const bus = createFakeEventBus<FsDeltaEvent>();
    const received: FsDeltaEvent[] = [];

    const unlisten = await subscribeSessionEvent<FsDeltaEvent>(
      "workspace://fs-delta",
      () => 0,
      (payload) => received.push(payload),
      bus.listenFn,
    );

    unlisten();
    expect(bus.unlisten).toHaveBeenCalledTimes(1);

    bus.emit({
      epoch: 0,
      payload: { deltas: [{ kind: "created", path: "after-unlisten.go" }] },
    });
    expect(received).toEqual([]);
  });
});
