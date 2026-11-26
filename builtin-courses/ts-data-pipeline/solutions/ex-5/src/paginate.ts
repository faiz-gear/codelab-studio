import type { Page } from "./types.ts";

export function paginateCursor<T extends { id: string }>(
  items: readonly T[],
  cursor: string | null,
  limit: number
): Page<T> {
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError("limit must be a positive integer");
  }
  let start = 0;
  if (cursor !== null) {
    const index = items.findIndex((item) => item.id === cursor);
    if (index === -1) {
      return { items: [], nextCursor: null, hasMore: false };
    }
    start = index + 1;
  }
  const pageItems = items.slice(start, start + limit);
  const hasMore = start + limit < items.length;
  const nextCursor =
    hasMore && pageItems.length > 0
      ? pageItems[pageItems.length - 1]!.id
      : null;
  return { items: pageItems, nextCursor, hasMore };
}
