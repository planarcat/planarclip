import type { UiSettingsPayload } from "../types";
import {
  applyAppearanceFromUiSettings,
  loadPreviewUiSettings,
  savePreviewUiSettings,
} from "./settings";

export { applyAppearanceFromUiSettings };

export function mirrorUiSettingsForBootstrap(settings: UiSettingsPayload) {
  savePreviewUiSettings(settings);
}

/** Apply cached appearance before React/CSS paint (main entry). */
export function bootstrapAppearanceFromLocalStorage() {
  applyAppearanceFromUiSettings(loadPreviewUiSettings());
}
