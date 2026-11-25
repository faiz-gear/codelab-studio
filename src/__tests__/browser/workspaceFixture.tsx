import "../../styles/tokens.css";
import "../../styles/app.css";
import "../../styles/runtime.css";
import "../../styles/workspace.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "../../App";
import { DEFAULT_SETTINGS } from "../../core/contracts";
import { FakeCoreClient } from "../fakeCoreClient";

declare global {
  interface Window {
    workspaceSmoke: {
      replaceMainFile(text: string): void;
      readMainFile(): string | null;
    };
  }
}

const root = "/work/monaco";
const client = new FakeCoreClient({
  settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
  pickedPaths: { workspaceDir: root },
  chapterSolutions: { "ex-0": { "main.go": "package main\n// reference implementation\n" } },
});
client.writeDiskFile(root, "main.go", "package main\n");
client.writeDiskFile(root, "other.go", "package second\n");

window.workspaceSmoke = {
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
