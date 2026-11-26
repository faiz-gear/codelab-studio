import { expectResponse, type CoreClient } from "../core/client";
import type { AppCommand, CommandConfirmation, CommandCwdOverrides, TerminalSummary, TerminalTarget } from "../core/contracts";
import type { WorkspaceEditorSession } from "../workspace/editorSession";

interface CommandRequest {
  commandId: string;
  restartTerminalId?: number;
  confirmation?: number;
}

interface WorkbenchState {
  cwd: CommandCwdOverrides;
  terminals: TerminalSummary[];
  active: number | null;
  confirmation: { request: CommandRequest; detail: CommandConfirmation } | null;
  busy: boolean;
  outputVersion: number;
}

export const emptyWorkbench: WorkbenchState = {
  cwd: {}, terminals: [], active: null, confirmation: null, busy: false, outputVersion: 0,
};

export class CommandWorkbench {
  private state = emptyWorkbench;
  private readonly listeners = new Set<() => void>();
  private listVersion = 0;
  private cwdVersion = 0;
  private readonly cursors = new Map<number, number>();

  constructor(private client: CoreClient, private editor: WorkspaceEditorSession) {}

  snapshot = () => this.state;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  private update(patch: Partial<WorkbenchState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  private get workspaceId() {
    return this.editor.workspace.id;
  }

  async loadCwd() {
    const version = ++this.cwdVersion;
    const reply = await this.client.dispatch({ type: "getCommandCwdOverrides", payload: { workspaceId: this.workspaceId } });
    if (version === this.cwdVersion) this.update({ cwd: expectResponse(reply, "commandCwdOverrides").payload });
  }

  async setCwd(commandId: string, cwd: string | null) {
    const reply = await this.client.dispatch({ type: "setCommandCwdOverride", payload: { workspaceId: this.workspaceId, commandId, cwd } });
    this.cwdVersion++;
    this.update({ cwd: expectResponse(reply, "commandCwdOverrides").payload });
  }

  async run(request: CommandRequest) {
    if (this.state.busy) return;
    this.update({ busy: true, confirmation: null });
    try {
      const response = await this.editor.runCommand(request);
      if (response?.type === "commandConfirmationRequired") {
        this.update({ confirmation: { request, detail: response.payload } });
      } else if (response?.type === "commandStarted") {
        this.show(response.payload.terminal);
      }
    } finally {
      this.update({ busy: false });
    }
  }

  dismissConfirmation() { this.update({ confirmation: null }); }

  async open() {
    if (this.state.busy) return;
    this.update({ busy: true });
    try {
      const reply = await this.client.dispatch({ type: "openTerminal", payload: { workspaceId: this.workspaceId } });
      this.show(expectResponse(reply, "terminalOpened").payload);
    } finally {
      this.update({ busy: false });
    }
  }

  private show(terminal: TerminalSummary) {
    this.listVersion++;
    const terminals = this.state.terminals.filter(({ id }) => id !== terminal.id);
    terminals.push(terminal);
    terminals.sort((first, second) => first.id - second.id);
    this.update({ active: terminal.id, terminals });
  }

  select(id: number) { this.update({ active: id }); }

  async updateTerminal(command: AppCommand) {
    expectResponse(await this.client.dispatch(command), "terminalUpdated");
    if (command.type === "closeTerminal") {
      const { terminalId } = command.payload;
      this.cursors.delete(terminalId);
      this.listVersion++;
      const terminals = this.state.terminals.filter(({ id }) => id !== terminalId);
      const active = this.state.active === terminalId ? terminals.at(-1)?.id ?? null : this.state.active;
      this.update({ terminals, active });
    }
  }

  target(terminalId: number): TerminalTarget { return { workspaceId: this.workspaceId, terminalId }; }

  async read(terminalId?: number, cursor?: number, includeRecentErrors = false) {
    const version = this.listVersion;
    const response = await this.client.dispatch({
      type: "readTerminals", payload: { workspaceId: this.workspaceId, terminalId, cursor, includeRecentErrors },
    });
    const updates = expectResponse(response, "terminals").payload;
    const terminalsChanged =
      version === this.listVersion &&
      JSON.stringify(updates.terminals) !== JSON.stringify(this.state.terminals);
    const hasNewOutput = Boolean(updates.output?.data) || Boolean(updates.output?.reset);
    if (terminalsChanged || hasNewOutput) {
      this.update({
        ...(terminalsChanged ? { terminals: updates.terminals } : {}),
        outputVersion: this.state.outputVersion + 1,
      });
    }
    return updates;
  }

  clearAt(id: number, cursor: number) { this.cursors.set(id, cursor); }
  startCursor(id: number) { return this.cursors.get(id) ?? 0; }
}
