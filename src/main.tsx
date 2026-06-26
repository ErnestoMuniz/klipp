import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import Overlay from "./Overlay.tsx";
import { applyTheme } from "./components/Soundboard/theme.ts";
import { loadPrefs } from "./components/Soundboard/types.ts";

const overlayMode = new URLSearchParams(window.location.search).has("overlay");
document.documentElement.classList.toggle("overlay-mode", overlayMode);

// Apply saved theme immediately to prevent flash
if (!overlayMode) {
  applyTheme(loadPrefs().theme);
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{overlayMode ? <Overlay /> : <App />}</StrictMode>,
);
