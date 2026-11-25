import { expectResponse, type CoreClient } from "../core/client";
import {
  CoreCommandError,
  type DomainError,
  type AppResponse,
  type RunCommandBlock,
  type ExternalIde,
  type WorkspaceComparison,
  type WorkspaceFile,
  type WorkspaceSummary,
} from "../core/contracts";

export interface EditorTab {
  file: WorkspaceFile;
  draft: string | null;
  conflict: WorkspaceFile | null;
  unavailable: DomainError | null;
}

export interface ComparisonReview {
  comparison: WorkspaceComparison;
  draft: string | null;
}

interface EditorState {
  tabs: EditorTab[];
  activePath: string | null;
  treeVersion: number;
  busy: boolean;
  error: DomainError | null;
  executionConflict: number;
}

export function editorText(tab: EditorTab): string | null {
  return tab.draft ?? (tab.file.kind === "text" ? tab.file.text : null);
}

export function domainError(error: unknown): DomainError {
  return error instanceof CoreCommandError ? error.domainError : { code: "UNEXPECTED_ERROR" };
}

function diskUpdate(tab: EditorTab, file: WorkspaceFile): EditorTab {
  if (tab.draft === null || (file.kind === "text" && tab.draft === file.text)) {
    return { file, draft: null, conflict: null, unavailable: null };
  }
  const conflict = tab.file.revision === file.revision ? null : file;
  return { ...tab, conflict, unavailable: null };
}

export class WorkspaceEditorSession {
  private listeners = new Set<() => void>();
  private queue: Promise<void> = Promise.resolve();
  private trusted = true;
  private state: EditorState = {
    tabs: [],
    activePath: null,
    treeVersion: 0,
    busy: false,
    error: null,
    executionConflict: 0,
  };

  constructor(private client: CoreClient, readonly workspace: WorkspaceSummary) {}

  snapshot = () => this.state;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  private update(patch: Partial<EditorState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  private target(path: string) {
    return { workspaceId: this.workspace.id, path };
  }

  private tab(path: string) {
    return this.state.tabs.find((tab) => tab.file.path === path);
  }

  private updateTab(path: string, update: (tab: EditorTab) => EditorTab) {
    this.update({ tabs: this.state.tabs.map((tab) => tab.file.path === path ? update(tab) : tab) });
  }

  private enqueue(operation: () => Promise<void>) {
    const result = this.queue.then(async () => {
      if (this.trusted) await operation();
    });
    this.queue = result.catch((error: unknown) => this.update({ error: domainError(error) }));
    return this.queue;
  }

  setTrusted(trusted: boolean) {
    if (this.trusted === trusted) return;
    this.trusted = trusted;
    if (trusted) void this.refresh([""]);
  }

  async list(path: string) {
    const response = await this.client.dispatch({ type: "listWorkspaceFiles", payload: this.target(path) });
    return expectResponse(response, "workspaceEntries").payload;
  }

  private async read(path: string) {
    const response = await this.client.dispatch({ type: "readWorkspaceFile", payload: this.target(path) });
    return expectResponse(response, "workspaceFile").payload;
  }

  select(path: string) {
    this.update({ activePath: path });
  }

  async open(path: string): Promise<string | null> {
    let opened: string | null = null;
    await this.enqueue(async () => {
      this.update({ error: null });
      const file = await this.read(path);
      if (file.kind === "missing") {
        throw new CoreCommandError({ code: "WORKSPACE_FILE_NOT_FOUND", details: { path } });
      }
      this.showFile(file);
      opened = file.path;
    });
    return opened;
  }

  private showFile(file: WorkspaceFile) {
    if (this.tab(file.path)) {
      this.updateTab(file.path, (tab) => diskUpdate(tab, file));
    } else {
      this.update({ tabs: [...this.state.tabs, { file, draft: null, conflict: null, unavailable: null }] });
    }
    this.select(file.path);
  }

  change(path: string, text: string) {
    this.updateTab(path, (tab) => {
      if (tab.file.kind === "text" && text === tab.file.text) {
        return diskUpdate({ ...tab, draft: null }, tab.conflict ?? tab.file);
      }
      return { ...tab, draft: text };
    });
  }

  resolveConflict(path: string, loadDisk: boolean) {
    this.updateTab(path, (tab) => {
      const file = tab.conflict;
      if (!file) return tab;
      return diskUpdate({ ...tab, file, draft: loadDisk ? null : tab.draft }, file);
    });
  }

  refresh(paths: string[]) {
    return this.enqueue(async () => {
      const targets = this.state.tabs.filter((tab) => paths.some((path) =>
        !path || tab.file.path === path || tab.file.path.startsWith(path + "/")
      ));
      await Promise.all(targets.map(async ({ file: { path } }) => {
        try {
          const file = await this.read(path);
          if (file.path !== path) {
            const error: DomainError = { code: "WORKSPACE_FILE_READ_FAILED", details: { path } };
            this.updateTab(path, (tab) => ({ ...tab, unavailable: error }));
          } else {
            this.updateTab(path, (tab) => diskUpdate(tab, file));
          }
        } catch (error) {
          this.updateTab(path, (tab) => ({ ...tab, unavailable: domainError(error) }));
        }
      }));
      this.update({ treeVersion: this.state.treeVersion + 1 });
    });
  }

  hasDrafts() {
    return this.state.tabs.some((tab) => tab.draft !== null);
  }

  async save(path?: string): Promise<boolean> {
    let saved = false;
    await this.enqueue(async () => {
      this.update({ busy: true, error: null });
      try {
        const dirty = this.state.tabs.filter((tab) => tab.draft !== null && (path === undefined || path === tab.file.path));
        for (const { file } of dirty) {
          if (!await this.saveTab(file.path)) {
            this.select(file.path);
            return;
          }
        }
        saved = path === undefined ? !this.hasDrafts() : this.tab(path)?.draft == null;
      } finally {
        this.update({ busy: false });
      }
    });
    return saved;
  }

  async runCommand(request: Omit<RunCommandBlock, "workspaceId" | "drafts">) {
    type RunResponse = Extract<AppResponse, { type: "commandStarted" | "commandConfirmationRequired" }>;
    const result: { response: RunResponse | null } = { response: null };
    await this.enqueue(async () => {
      this.update({ busy: true, error: null });
      try {
        const drafts = this.state.tabs.flatMap(({ file, draft }) => draft === null ? [] : [{
          path: file.path, text: draft, expectedRevision: file.revision,
        }]);
        const response = await this.client.dispatch({
          type: "runCommandBlock",
          payload: { ...request, workspaceId: this.workspace.id, drafts },
        });
        if (response.type === "commandConfirmationRequired") {
          result.response = response;
          return;
        }
        const outcome = response.type === "commandSaveConflict" ? response : expectResponse(response, "commandStarted");
        for (const file of outcome.payload.saved) {
          this.updateTab(file.path, (tab) => diskUpdate({ ...tab, file }, file));
        }
        if (outcome.type === "commandSaveConflict") {
          const file = outcome.payload.file;
          this.updateTab(file.path, (tab) => diskUpdate(tab, file));
          this.update({ activePath: file.path, executionConflict: this.state.executionConflict + 1 });
        } else {
          result.response = outcome;
        }
      } finally {
        this.update({ busy: false });
      }
    });
    return result.response;
  }

  private async saveTab(path: string): Promise<boolean> {
    const tab = this.tab(path);
    if (!tab || tab.draft === null) return true;
    if (tab.conflict || tab.unavailable) return false;
    try {
      const response = await this.client.dispatch({
        type: "writeWorkspaceFile",
        payload: { ...this.target(path), text: tab.draft, expectedRevision: tab.file.revision },
      });
      if (response.type === "workspaceFileConflict") {
        this.acceptDiskUpdate(path, response.payload);
        return this.tab(path)?.draft === null;
      }
      const file = expectResponse(response, "workspaceFile").payload;
      this.updateTab(path, (current) => diskUpdate({ ...current, file }, file));
      return true;
    } catch (error) {
      this.update({ error: domainError(error) });
      return false;
    }
  }

  private acceptDiskUpdate(path: string, file: WorkspaceFile) {
    this.updateTab(path, (tab) => file.path === path
      ? diskUpdate(tab, file)
      : { ...tab, unavailable: { code: "WORKSPACE_FILE_READ_FAILED", details: { path } } });
  }

  async previewChapterSolution(chapterId: string): Promise<ComparisonReview[]> {
    const response = await this.client.dispatch({
      type: "previewChapterSolution",
      payload: { workspaceId: this.workspace.id, chapterId },
    });
    return expectResponse(response, "workspaceComparisons").payload.map((file) => this.review(file));
  }

  async listStarterFiles(): Promise<string[]> {
    const response = await this.client.dispatch({
      type: "listStarterFiles", payload: { workspaceId: this.workspace.id },
    });
    return expectResponse(response, "starterFiles").payload;
  }

  async previewStarterFile(path: string): Promise<ComparisonReview | undefined> {
    const response = await this.client.dispatch({ type: "previewStarterFile", payload: this.target(path) });
    return expectResponse(response, "workspaceComparisons").payload.map((file) => this.review(file))[0];
  }

  private review(comparison: WorkspaceComparison): ComparisonReview {
    return { comparison, draft: this.tab(comparison.current.path)?.draft ?? null };
  }

  async applyComparison({ comparison, draft }: ComparisonReview): Promise<"applied" | "conflict" | "stale" | "failed"> {
    const { current, proposed } = comparison;
    const { path } = current;
    let outcome: "applied" | "conflict" | "stale" | "failed" = "failed";
    await this.enqueue(async () => {
      this.update({ busy: true, error: null });
      try {
        const tab = this.tab(path);
        if ((tab?.draft ?? null) !== draft) {
          outcome = "stale";
          return;
        }
        if (tab?.conflict || tab?.unavailable) {
          this.select(path);
          outcome = "conflict";
          return;
        }
        // The preview may see an external write before the watcher delivers it.
        if (tab && draft !== null && tab.file.revision !== current.revision) {
          const disk = await this.read(path);
          this.acceptDiskUpdate(path, disk);
          this.select(path);
          outcome = this.tab(path)?.conflict ? "conflict" : "stale";
          return;
        }
        const response = await this.client.dispatch({
          type: "applyWorkspaceComparison",
          payload: { ...this.target(path), text: proposed, expectedRevision: current.revision },
        });
        if (response.type === "workspaceFileConflict") {
          this.acceptDiskUpdate(path, response.payload);
          if (this.tab(path)?.conflict) {
            this.select(path);
            outcome = "conflict";
          } else {
            outcome = "stale";
          }
          return;
        }
        const file = expectResponse(response, "workspaceFile").payload;
        this.updateTab(path, (previous) => {
          const nextDraft = previous.draft === draft ? null : previous.draft;
          return diskUpdate({ ...previous, file, draft: nextDraft }, file);
        });
        this.showFile(file);
        this.update({ treeVersion: this.state.treeVersion + 1 });
        outcome = "applied";
      } finally {
        this.update({ busy: false });
      }
    });
    return outcome;
  }

  close(path: string) {
    const index = this.state.tabs.findIndex((tab) => tab.file.path === path);
    const tabs = this.state.tabs.filter((tab) => tab.file.path !== path);
    const activePath = this.state.activePath === path
      ? tabs[Math.min(index, tabs.length - 1)]?.file.path ?? null
      : this.state.activePath;
    this.update({ tabs, activePath });
  }

  private async editTree(operation: () => Promise<void>) {
    let done = false;
    await this.enqueue(async () => {
      this.update({ busy: true, error: null });
      try {
        await operation();
        this.update({ treeVersion: this.state.treeVersion + 1 });
        done = true;
      } finally {
        this.update({ busy: false });
      }
    });
    return done;
  }

  create(path: string) {
    return this.editTree(async () => {
      const response = await this.client.dispatch({ type: "createWorkspaceFile", payload: this.target(path) });
      const file = expectResponse(response, "workspaceFile").payload;
      this.showFile(file);
    });
  }

  rename(path: string, newPath: string) {
    return this.editTree(async () => {
      const response = await this.client.dispatch({
        type: "renameWorkspaceFile", payload: { ...this.target(path), newPath },
      });
      const file = expectResponse(response, "workspaceFile").payload;
      const destination = file.path === path ? undefined : this.tab(file.path);
      if (destination && destination.draft !== null) {
        // The open destination had been deleted externally. Keep its draft in that tab.
        // A source draft keeps its original tab if both versions need attention.
        this.updateTab(file.path, (tab) => diskUpdate(tab, file));
        const source = this.tab(path);
        if (source && source.draft !== null) {
          this.updateTab(path, (tab) => diskUpdate(tab, { kind: "missing", path, revision: null, sizeBytes: 0 }));
        } else {
          this.close(path);
          this.select(file.path);
        }
      } else {
        if (destination) this.close(file.path);
        this.updateTab(path, (tab) => {
          const unchanged = tab.file.kind === "text" && file.kind === "text" && tab.file.text === file.text;
          const baseline = unchanged ? file : { ...tab.file, path: file.path };
          return diskUpdate({ ...tab, file: baseline }, file);
        });
        if (this.state.activePath === path) this.select(file.path);
      }
    });
  }

  openIde(path: string, ide: ExternalIde) {
    return this.enqueue(async () => {
      this.update({ error: null });
      const response = await this.client.dispatch({
        type: "openWorkspaceIde", payload: { ...this.target(path), ide },
      });
      expectResponse(response, "workspaceIdeOpened");
    });
  }
}
