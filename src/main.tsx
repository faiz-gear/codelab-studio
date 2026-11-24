import "@fontsource/geist/400.css";
import "@fontsource/geist/500.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./styles/tokens.css";
import "./styles/app.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { FakeCoreClient } from "./core/fakeClient";
import { isTauriRuntime, TauriCoreClient } from "./core/tauriClient";

const client = isTauriRuntime()
  ? new TauriCoreClient()
  : new FakeCoreClient({ persistKey: "codelab.browser-preview.v1" });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App client={client} />
  </StrictMode>
);
