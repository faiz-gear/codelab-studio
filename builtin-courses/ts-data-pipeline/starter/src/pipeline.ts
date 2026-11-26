import type { NormalizedEvent, PipelineSummary, RawEvent } from "./types.ts";

// 练习 8：组合 normalizeEvents -> dedupKeepLatest -> runWithConcurrency -> summarizeByCategory
export async function runDataPipeline(
  raw: readonly RawEvent[],
  limit: number,
  enrich: (event: NormalizedEvent) => Promise<NormalizedEvent>
): Promise<PipelineSummary> {
  // TODO: 串联各阶段并计算 topCategory
  void raw;
  void limit;
  void enrich;
  return {
    processedCount: 0,
    categories: {},
    topCategory: null,
  };
}
