import type { Page } from "./types.ts";

// 练习 5：实现确定性游标分页，校验 limit 为正整数并正确计算 nextCursor 与 hasMore
export function paginateCursor<T extends { id: string }>(
  items: readonly T[],
  cursor: string | null,
  limit: number
): Page<T> {
  // TODO: 按游标与分页大小切片
  void cursor;
  void limit;
  return {
    items: [...items],
    nextCursor: null,
    hasMore: false,
  };
}
