// Choosing "Fleet settings" from the command palette goes to the Settings screen.
//
// **This is the wiring the palette's Settings section exists to prove.** The
// primitive draws the row and `@armada/shell` turns choosing it into a
// `PaletteChoice`; neither package can mount `BridgeSettings`, since a screen
// sits above both on the layer this package's own comment names. So the
// claim — a chosen row actually navigates, not only that something fired —
// is provable only here, where `Palette` and the screen can be mounted side
// by side.

import { afterEach, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { expect } from "vitest";
import { useState } from "react";

import { Palette } from "@armada/shell";
import type { FleetLimits, Outcome } from "@armada/protocol";

import { BridgeSettings } from "./BridgeSettings";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const LIMITS: FleetLimits = {
  concurrency: 2,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  checks_at_once: 4,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10, checks_at_once: 4 },
};

/** A stand-in for the app: the palette and the rail's own toggle, wired the way `App.tsx` wires them. */
function Host() {
  const [showing, setShowing] = useState(false);
  return (
    <>
      <Palette
        open
        onClose={() => {}}
        context="board"
        on={null}
        surfaces={[]}
        jobs={[]}
        settings={[{ id: "fleet_settings", label: "Fleet settings" }]}
        onChoose={(choice) => {
          if (choice.of === "setting" && choice.id === "fleet_settings") setShowing(true);
        }}
        onConfirmAct={() => {}}
      />
      {showing ? (
        <BridgeSettings
          limits={LIMITS}
          live
          onSave={(): Promise<Outcome> => Promise.resolve({ ok: true })}
        />
      ) : null}
    </>
  );
}

test("choosing Fleet settings from the palette shows the Settings screen", async () => {
  mount(<Host />);

  await expect.element(page.getByRole("heading", { name: "Fleet" })).not.toBeInTheDocument();

  await userEvent.click(page.getByRole("option", { name: "Fleet settings" }));

  await expect.element(page.getByRole("heading", { name: "Fleet" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "This machine" })).toBeVisible();
});
