import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./styles/index.css";
import { App } from "./App";

// Bridge's renderer entry point. No Node, no `require`, no socket — everything
// it draws arrives through the preload from the one connection in the main
// process.
const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
