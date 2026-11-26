import type { CategorySummary, NormalizedEvent } from "./types.ts";

export function summarizeByCategory(
  events: readonly NormalizedEvent[]
): Record<string, CategorySummary> {
  const summaries: Record<string, CategorySummary> = {};
  for (const event of events) {
    const current = summaries[event.category];
    if (!current) {
      summaries[event.category] = {
        count: 1,
        totalAmount: Math.round(event.amount * 100) / 100,
        maxLatencyMs: event.latencyMs,
      };
    } else {
      current.count += 1;
      current.totalAmount =
        Math.round((current.totalAmount + event.amount) * 100) / 100;
      current.maxLatencyMs = Math.max(current.maxLatencyMs, event.latencyMs);
    }
  }
  return summaries;
}
