import React from "react";
import { createRoot } from "react-dom/client";

import { CrateCatalog } from "./CrateCatalog";
import "./index.css";

function App() {
  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <CrateCatalog />
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
