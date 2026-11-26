import type { NormalizedEvent } from "./types.ts";

export function dedupKeepLatest(
  events: readonly NormalizedEvent[]
): NormalizedEvent[] {
  const latestById = new Map<string, NormalizedEvent>();
  for (const event of events) {
    const existing = latestById.get(event.id);
    if (!existing || event.timestamp >= existing.timestamp) {
      latestById.set(event.id, event);
    }
  }
  return Array.from(latestById.values()).sort((left, right) => {
    if (left.timestamp !== right.timestamp) {
      return left.timestamp - right.timestamp;
    }
    return left.id.localeCompare(right.id);
  });
}
