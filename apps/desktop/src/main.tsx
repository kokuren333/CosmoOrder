import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";
import "./styles/app.css";
import "katex/dist/katex.min.css";
import "highlight.js/styles/github.css";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("the Osmium root element is missing");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
