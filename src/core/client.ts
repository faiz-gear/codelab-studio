import type {
  AppBootstrap,
  AppCommand,
  AppEvent,
  AppResponse,
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

export async function openCourse(
  client: CoreClient,
  request: OpenCourseRequest
): Promise<LoadedCourse> {
  const response = await client.dispatch({
    type: "openCourse",
    payload: request,
  });
  if (response.type !== "loadedCourse") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}

export async function reloadActiveCourse(
  client: CoreClient
): Promise<LoadedCourse> {
  const response = await client.dispatch({ type: "reloadActiveCourse" });
  if (response.type !== "loadedCourse") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
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
  if (response.type !== "preflight") {
    throw new Error(`Unexpected response type: ${response.type}`);
  }
  return response.payload;
}
