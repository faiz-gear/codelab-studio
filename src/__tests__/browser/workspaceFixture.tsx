import "../../styles/tokens.css";
import "../../styles/app.css";
import "../../styles/runtime.css";
import "../../styles/workspace.css";
import "../../styles/execution.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "../../App";
import { DEFAULT_SETTINGS, type AppCommand, type AppResponse, type LanguageFeatureResult, type LearningProgress } from "../../core/contracts";
import { FakeCoreClient } from "../fakeCoreClient";

declare global {
  interface Window {
    workspaceSmoke: {
      replaceMainFile(text: string): void;
      readMainFile(): string | null;
      writeTerminalOutput(text: string): void;
      terminalInput(): string;
      setCheckProgress(progress: LearningProgress): void;
    };
  }
}

const root = "/work/monaco";
class BrowserCoreClient extends FakeCoreClient {
  override async dispatch(command: AppCommand): Promise<AppResponse> {
    const response = await super.dispatch(command);
    if (!new URLSearchParams(location.search).has("language-server")) return response;
    if (command.type === "syncLanguageDocument" && command.payload.document) {
      const { workspaceId, version, document } = command.payload;
      const target = { workspaceId, version, path: document.path };
      this.emitEvent({ type: "languageStatusChanged", payload: { ...target, language: "Go", status: "ready" } });
      this.emitEvent({ type: "languageDiagnostics", payload: { ...target, diagnostics: [{
        message: "Fixture diagnostic", severity: 2,
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 7 } },
      }] } });
    } else if (command.type === "requestLanguageFeature") {
      const payload: LanguageFeatureResult = command.payload.feature === "completion"
        ? { kind: "completion", value: { items: [{ label: "mainFixture", kind: 3 }] } }
        : { kind: "hover", value: { contents: [{ kind: "markdown", value: "**Fixture documentation**" }] } };
      return { type: "languageFeature", payload };
    }
    return response;
  }
}

const client = new BrowserCoreClient({
  settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
  pickedPaths: { workspaceDir: root },
  chapterSolutions: { "ex-0": { "main.go": "package main\n// reference implementation\n" } },
});
client.writeDiskFile(root, "main.go", "package main\n");
client.writeDiskFile(root, "other.go", "package second\n");

window.workspaceSmoke = {
  setCheckProgress(progress) { client.emitEvent({type:"progressUpdated",payload:progress}); },
  writeTerminalOutput(text) { client.writeTerminalOutput(text); },
  terminalInput() {
    return client.commandLog.flatMap((command) => command.type === "writeTerminal" ? [command.payload.text] : []).join("");
  },
  replaceMainFile(text) {
    client.writeDiskFile(root, "main.go", text);
  },
  readMainFile() {
    const file = client.readDiskFile(root, "main.go");
    return file?.kind === "text" ? file.text : null;
  },
};

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App client={client} />
  </StrictMode>
);
