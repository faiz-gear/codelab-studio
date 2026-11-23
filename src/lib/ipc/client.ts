import { invoke } from "@tauri-apps/api/core";
import type { BootstrapState } from "./generated";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function appBootstrap(): Promise<BootstrapState> {
  return invoke<BootstrapState>("app_bootstrap");
}
