import type { NormalizedEvent, RawEvent } from "./types.ts";

export function normalizeEvents(events: readonly RawEvent[]): NormalizedEvent[] {
  const result: NormalizedEvent[] = [];
  for (const event of events) {
    const id = event.id.trim();
    const category = event.category.trim().toLowerCase();
    if (!id || !category) {
      continue;
    }
    const seenTags = new Set<string>();
    const tags: string[] = [];
    for (const rawTag of event.tags) {
      const tag = rawTag.trim().toLowerCase();
      if (tag && !seenTags.has(tag)) {
        seenTags.add(tag);
        tags.push(tag);
      }
    }
    result.push({
      id,
      category,
      amount: event.amount,
      latencyMs: Math.max(0, event.latencyMs),
      timestamp: event.timestamp,
      tags,
    });
  }
  return result;
}
