import type { UserRecord } from "./types.ts";

// 练习 1：筛出 age >= 18 的成年用户，仅返回 name 数组且不修改原数组
export function getAdultNames(users: readonly UserRecord[]): string[] {
  // TODO: 仅保留 age >= 18 的用户并映射出 name
  return users.map((user) => user.name);
}
