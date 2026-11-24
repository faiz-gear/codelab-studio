import type {
  AppBootstrap,
  AppCommand,
  AppEvent,
  AppResponse,
  GlobalSettings,
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
