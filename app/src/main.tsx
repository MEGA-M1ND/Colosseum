import { Buffer } from "buffer";
// web3.js and the instruction encoders assume Node's Buffer.
globalThis.Buffer = globalThis.Buffer ?? Buffer;

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
