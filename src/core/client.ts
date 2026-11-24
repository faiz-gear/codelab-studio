import type {
  AppBootstrap,
  AppCommand,
  AppEvent,
  AppResponse,
  GlobalSettings,
  RecentWorkspace,
  ShutdownReport,
  TrustEntry,
  TrustStatus,
  UpdateSettingsPatch,
} from "./contracts";

export type CoreEventListener = (event: AppEvent) => void;
export type Unsubscribe = () => void;

export interface CoreClient {
  dispatch(command: AppCommand): Promise<AppResponse>;
  subscribe(listener: CoreEventListener): Promise<Unsubscribe>;
}

export async function fetchBootstrap(client: CoreClient): Promise<AppBootstrap> {
  const response = await client.dispatch({ type: "bootstrap" });
  if (response.type !== "bootstrap") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function updateSettings(
  client: CoreClient,
  patch: UpdateSettingsPatch
): Promise<GlobalSettings> {
  const response = await client.dispatch({
    type: "updateSettings",
    payload: patch,
  });
  if (response.type !== "settings") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function listRecents(
  client: CoreClient
): Promise<RecentWorkspace[]> {
  const response = await client.dispatch({ type: "listRecents" });
  if (response.type !== "recents") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function listTrustEntries(
  client: CoreClient
): Promise<TrustEntry[]> {
  const response = await client.dispatch({ type: "listTrustEntries" });
  if (response.type !== "trustEntries") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function checkTrust(
  client: CoreClient,
  sourceKind: TrustStatus["sourceKind"],
  source: string
): Promise<TrustStatus> {
  const response = await client.dispatch({
    type: "checkTrust",
    payload: { sourceKind, source },
  });
  if (response.type !== "trustStatus") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function requestShutdown(
  client: CoreClient
): Promise<ShutdownReport> {
  const response = await client.dispatch({ type: "shutdown" });
  if (response.type !== "shutdown") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}
