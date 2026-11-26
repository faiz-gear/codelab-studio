import { summarizeByCategory } from "./aggregate.ts";
import { runWithConcurrency } from "./concurrency.ts";
import { dedupKeepLatest } from "./dedup.ts";
import { normalizeEvents } from "./normalize.ts";
import type { NormalizedEvent, PipelineSummary, RawEvent } from "./types.ts";

export async function runDataPipeline(
  raw: readonly RawEvent[],
  limit: number,
  enrich: (event: NormalizedEvent) => Promise<NormalizedEvent>
): Promise<PipelineSummary> {
  const normalized = normalizeEvents(raw);
  const deduped = dedupKeepLatest(normalized);
  const enriched = await runWithConcurrency(deduped, limit, enrich);
  const categories = summarizeByCategory(enriched);

  let topCategory: string | null = null;
  let topAmount = -Infinity;
  for (const category of Object.keys(categories).sort()) {
    const summary = categories[category]!;
    if (summary.totalAmount > topAmount) {
      topAmount = summary.totalAmount;
      topCategory = category;
    }
  }

  return {
    processedCount: enriched.length,
    categories,
    topCategory,
  };
}
