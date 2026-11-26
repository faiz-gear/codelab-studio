import type { NormalizedEvent } from "./types.ts";

// 练习 3：按 id 去重保留 timestamp 最大的事件（平手保留靠后者），最终按 timestamp 升序、id 升序返回
export function dedupKeepLatest(
  events: readonly NormalizedEvent[]
): NormalizedEvent[] {
  // TODO: 实现去重与稳定排序
  return [...events];
}
