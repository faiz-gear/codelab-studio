import type { UserRecord } from "./types.ts";

export function getAdultNames(users: readonly UserRecord[]): string[] {
  return users.filter((user) => user.age >= 18).map((user) => user.name);
}
