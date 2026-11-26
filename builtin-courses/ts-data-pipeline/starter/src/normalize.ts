import type { NormalizedEvent, RawEvent } from "./types.ts";

// 练习 2：清洗原始事件：trim、category/tags 转小写、丢弃空 id/category、负延迟钳制为 0、去重标签
export function normalizeEvents(events: readonly RawEvent[]): NormalizedEvent[] {
  // TODO: 按规则清洗并归一化事件列表
  return events.map((event) => ({
    ...event,
    tags: [...event.tags],
  }));
}
