// Choosing "Fleet settings" from the command palette opens the sheet.
//
// **This is the wiring the palette's Settings section exists to prove.** The
// primitive draws the row and `@armada/shell` turns choosing it into a
// `PaletteChoice`; neither package can mount `FleetSettingsSheet`, since a
// screen sits above both on the layer this package's own comment names. So
// the claim — a chosen row actually opens the sheet, not only that something
// fired — is provable only here, where `Palette` and the sheet can be mounted
// side by side.

import { afterEach, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { expect } from "vitest";
import { useState } from "react";

import { Palette } from "@armada/shell";
import type { FleetLimits, Outcome } from "@armada/protocol";

import { FleetSettingsSheet } from "./FleetSettings";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const LIMITS: FleetLimits = {
  concurrency: 2,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10 },
};

/** A stand-in for the app: the palette and the sheet, wired the way `App.tsx` wires them. */
function Host() {
  const [open, setOpen] = useState(false);
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
          if (choice.of === "setting" && choice.id === "fleet_settings") setOpen(true);
        }}
        onConfirmAct={() => {}}
      />
      {open ? (
        <FleetSettingsSheet
          limits={LIMITS}
          live
          floor={false}
          onClose={() => setOpen(false)}
          onSave={(): Promise<Outcome> => Promise.resolve({ ok: true })}
        />
      ) : null}
    </>
  );
}

test("choosing Fleet settings from the palette opens the sheet", async () => {
  mount(<Host />);

  await expect.element(page.getByRole("dialog", { name: "Fleet settings" })).not.toBeInTheDocument();

  await userEvent.click(page.getByRole("option", { name: "Fleet settings" }));

  await expect.element(page.getByRole("dialog", { name: "Fleet settings" })).toBeVisible();
});
