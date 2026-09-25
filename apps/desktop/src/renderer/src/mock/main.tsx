// The mock's page: the app on `?scenario=<name>`, and the picker beside it.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { isGuideShape } from "@armada/components";

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

// `?guides=prose|steps|figure` — three shapes of one guide, for the owner to
// choose between. **Read here and nowhere else**: it is a comparison in the
// mock, not a setting in Bridge, and the app's own `main.tsx` mounts no
// provider. Anything unrecognised is prose, the same way an unknown scenario
// falls back rather than failing.
const shaped = query.get("guides");
const guides = shaped !== null && isGuideShape(shaped) ? shaped : "prose";
if (shaped !== null && shaped !== guides) {
  console.warn(`no guide shape named ${shaped}; showing ${guides}`);
}

const root = document.getElementById("root");
const picker = document.getElementById("picker");
if (root !== null && picker !== null) {
  mountApp(scenario, root, undefined, guides);
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
