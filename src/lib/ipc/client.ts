import { invoke } from "@tauri-apps/api/core";
import type {
  BootstrapState,
  GlobalConfigPatch,
  GlobalConfigView,
  MetricEvent,
  SessionEpoch,
} from "./generated";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function appBootstrap(): Promise<BootstrapState> {
  return invoke<BootstrapState>("app_bootstrap");
}

export async function settingsUpdate(patch: GlobalConfigPatch): Promise<GlobalConfigView> {
  return invoke<GlobalConfigView>("settings_update", { patch });
}

export async function metricsRecord(
  epoch: SessionEpoch | null,
  event: MetricEvent,
): Promise<void> {
  return invoke<void>("metrics_record", { epoch, event });
}

export async function workspaceClose(epoch: SessionEpoch): Promise<void> {
  return invoke<void>("workspace_close", { epoch });
}
