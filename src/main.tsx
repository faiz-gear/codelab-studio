import "@fontsource/geist/400.css";
import "@fontsource/geist/500.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./styles/loadStyles";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { TauriCoreClient } from "./core/tauriClient";

const client = new TauriCoreClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App client={client} />
  </StrictMode>
);
