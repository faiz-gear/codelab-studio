// 练习 7：限制最大并发数为 limit 且保持输出顺序与输入一致
export async function runWithConcurrency<T, R>(
  items: readonly T[],
  limit: number,
  worker: (item: T) => Promise<R>
): Promise<R[]> {
  // TODO: 校验 limit 并限制同时在途任务数不超过 limit
  void limit;
  return Promise.all(items.map((item) => worker(item)));
}
