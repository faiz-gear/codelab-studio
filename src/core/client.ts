import type {
  AppBootstrap,
  AppResponse,
  AppCommand,
  AppEvent,
  GlobalSettings,
  PreflightCommand,
  PreflightReport,
  LoadedCourse,
  LocalPathPickKind,
  OpenCourseRequest,
  UpdateSettingsPatch,
} from "./contracts";

export type CoreEventListener = (event: AppEvent) => void;
export type Unsubscribe = () => void;

export interface CoreClient {
  dispatch(command: AppCommand): Promise<AppResponse>;
  subscribe(listener: CoreEventListener): Promise<Unsubscribe>;
  pickLocalPath?(kind: LocalPathPickKind): Promise<string | null>;
}

export async function fetchBootstrap(client: CoreClient): Promise<AppBootstrap> {
  const response = await client.dispatch({ type: "bootstrap" });
  return expectResponse(response, "bootstrap").payload;
}

export async function updateSettings(
  client: CoreClient,
  patch: UpdateSettingsPatch
): Promise<GlobalSettings> {
  const response = await client.dispatch({
    type: "updateSettings",
    payload: patch,
  });
  return expectResponse(response, "settings").payload;
}

export async function openCourse(
  client: CoreClient,
  request: OpenCourseRequest
): Promise<LoadedCourse> {
  const response = await client.dispatch({
    type: "openCourse",
    payload: request,
  });
  return expectResponse(response, "loadedCourse").payload;
}

export async function reloadActiveCourse(
  client: CoreClient
): Promise<LoadedCourse> {
  const response = await client.dispatch({ type: "reloadActiveCourse" });
  return expectResponse(response, "loadedCourse").payload;
}

export async function closeActiveCourse(client: CoreClient): Promise<void> {
  const response = await client.dispatch({ type: "closeActiveCourse" });
  if (response.type !== "courseClosed") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
}


export async function dispatchPreflight(
  client: CoreClient,
  command: PreflightCommand,
): Promise<PreflightReport> {
  const response = await client.dispatch(command);
  return expectResponse(response, "preflight").payload;
}

export function expectResponse<T extends AppResponse["type"]>(
  response: AppResponse,
  type: T,
): Extract<AppResponse, { type: T }> {
  if (response.type !== type) {
    throw new Error("Unexpected core response " + response.type);
  }
  return response as Extract<AppResponse, { type: T }>;
}
