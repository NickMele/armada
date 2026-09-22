// Which scenario the mock is on, and a way to another. Dev-only: nothing the
// Electron build bundles imports this file.

import { Card, Select } from "@armada/components";

import { SCENARIOS } from "./scenario";
import type { Scenario } from "./scenario";

/**
 * The scenarios grouped by what comes before the first `/`, in the order
 * `SCENARIOS` lists them. **A name with no `/` is its own first group** — the
 * whole-app moments are what the mock opens on, and burying them under a
 * heading would put `every-state` below a dozen `arc/…` rows.
 */
function grouped(): [string, Scenario[]][] {
  const groups = new Map<string, Scenario[]>();
  for (const one of SCENARIOS) {
    const at = one.name.indexOf("/");
    const key = at === -1 ? "" : one.name.slice(0, at);
    const held = groups.get(key);
    if (held === undefined) groups.set(key, [one]);
    else held.push(one);
  }
  return [...groups.entries()];
}

/** Choosing reloads on `?scenario=`, so a scenario never inherits the last one's window state. */
export function Picker({ current }: { current: string }) {
  return (
    <Card className="armada-mock-picker">
      <Select
        label="Mock scenario"
        value={current}
        onChange={(event) => {
          const url = new URL(window.location.href);
          url.searchParams.set("scenario", event.target.value);
          window.location.assign(url);
        }}
      >
        {grouped().map(([group, scenarios]) =>
          group === "" ? (
            scenarios.map((one) => (
              <option key={one.name} value={one.name} title={one.says}>
                {one.name}
              </option>
            ))
          ) : (
            <optgroup key={group} label={group}>
              {scenarios.map((one) => (
                <option key={one.name} value={one.name} title={one.says}>
                  {one.name}
                </option>
              ))}
            </optgroup>
          ),
        )}
      </Select>
    </Card>
  );
}
