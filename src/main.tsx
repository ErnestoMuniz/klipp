import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import Overlay from "./Overlay.tsx";

const overlayMode = new URLSearchParams(window.location.search).has("overlay");
document.documentElement.classList.toggle("overlay-mode", overlayMode);

createRoot(document.getElementById("root")!).render(
  <StrictMode>{overlayMode ? <Overlay /> : <App />}</StrictMode>,
);
