import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./app/bootstrapAppearanceEntry";
import App from "./app/App.tsx";
import { installGlobalErrorCapture } from "./app/utils/logger";
import "./styles/index.css";

installGlobalErrorCapture();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
