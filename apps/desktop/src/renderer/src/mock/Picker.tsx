// Which scenario the mock is on, and a way to another. Dev-only: nothing the
// Electron build bundles imports this file.

import { Card, Select } from "@armada/components";

import { SCENARIOS } from "./scenario";

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
        {SCENARIOS.map((one) => (
          <option key={one.name} value={one.name} title={one.says}>
            {one.name}
          </option>
        ))}
      </Select>
    </Card>
  );
}
