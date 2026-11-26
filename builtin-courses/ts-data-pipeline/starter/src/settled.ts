import type { BatchResult } from "./types.ts";

// 练习 6：使用 Promise.allSettled 收集成功结果与失败消息，不因单项失败而中断
export async function collectSettled<T, R>(
  items: readonly T[],
  worker: (item: T) => Promise<R>
): Promise<BatchResult<R>> {
  // TODO: 替换为 Promise.allSettled 容错收集
  void items;
  void worker;
  return { fulfilled: [], rejected: [] };
}
