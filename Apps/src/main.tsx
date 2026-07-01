import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import "./app/bootstrapAppearanceEntry";
import App from "./app/App.tsx";
import {
  applyAppearanceFromUiSettings,
  mirrorUiSettingsForBootstrap,
} from "./app/utils/appearanceBootstrap";
import type { UiSettingsPayload } from "./app/types";
import "./styles/index.css";

async function startApp() {
  if (isTauri()) {
    try {
      const uiSettings = await invoke<UiSettingsPayload>("get_ui_settings");
      applyAppearanceFromUiSettings(uiSettings);
      mirrorUiSettingsForBootstrap(uiSettings);
    } catch {
      // Keep localStorage bootstrap; connection bridge will retry.
    }
  }

  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void startApp();
