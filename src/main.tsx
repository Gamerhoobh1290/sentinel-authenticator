import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

// Sentinel Authenticator — React entry point.
// The vault is NOT loaded here. The lock screen gates access; only after
// successful unlock does the decrypted view mount.
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
