// The mock's page: the app on `?scenario=<name>`, and the picker beside it.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./mock.css";
import { mountApp } from "./mount";
import { Picker } from "./Picker";
import { SCENARIOS, scenarioNamed } from "./scenario";

const query = new URLSearchParams(window.location.search);
const asked = query.get("scenario");
// `?frame` draws the app alone, for Evidence to photograph: no picker over it.
const framing = query.has("frame");
const scenario = (asked === null ? undefined : scenarioNamed(asked)) ?? SCENARIOS[0]!;
if (asked !== null && asked !== scenario.name) {
  console.warn(`no mock scenario named ${asked}; showing ${scenario.name}`);
}

const root = document.getElementById("root");
const picker = document.getElementById("picker");
if (root !== null && picker !== null) {
  mountApp(scenario, root);
  if (!framing) {
    createRoot(picker).render(
      <StrictMode>
        <Picker current={scenario.name} />
      </StrictMode>,
    );
  }
}

// The annotation layer (#1226), saving through this dev server's `annotationsServer`. Not in a frame.
if (!framing) void import("../annotate/mount").then(({ mount }) => mount());
