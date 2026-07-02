import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./app/bootstrapAppearanceEntry";
import App from "./app/App.tsx";
import "./styles/index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
