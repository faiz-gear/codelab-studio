import type { BatchResult } from "./types.ts";

export async function collectSettled<T, R>(
  items: readonly T[],
  worker: (item: T) => Promise<R>
): Promise<BatchResult<R>> {
  const outcomes = await Promise.allSettled(items.map((item) => worker(item)));
  const fulfilled: R[] = [];
  const rejected: string[] = [];
  for (const outcome of outcomes) {
    if (outcome.status === "fulfilled") {
      fulfilled.push(outcome.value);
    } else {
      const reason = outcome.reason;
      rejected.push(reason instanceof Error ? reason.message : String(reason));
    }
  }
  return { fulfilled, rejected };
}
