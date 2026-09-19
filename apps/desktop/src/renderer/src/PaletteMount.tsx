// What the palette is handed, and what it hands back — #1528. Lifted out of
// `App.tsx` whole, beside `palette.ts`, which already holds what the palette
// can reach and what is dormant where.
//
// **The acts travel as one object** because `carryOut` already takes them as
// one. Ten props would be ten chances for a caller to wire one to the wrong
// thing; the object is the shape that function already agrees on.

import { Palette, type PaletteSurface } from "@armada/shell";
import type { JobSummary } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";
import { BOARD_TABS, checkoutRunnablesOf, studioName, type BoardTab } from "@armada/screens";
import { absentIn, carryOut, dormantIn } from "./palette";

/** The Job or Studio the palette's rows act on, as `App.tsx` resolved it. */
export type PaletteOn = { id: string; title: string } | undefined;

export type PaletteMountProps = {
  open: boolean;
  onClose: () => void;
  /** A Job read whole, or nothing. Decides the context and the filters. */
  reading: JobSummary | null;
  /** The Studio open on its whiteboard, or nothing — by its name alone, which
      is all a title needs and all this should be able to read. */
  shownStudio: { name?: string } | null;
  on: PaletteOn;
  surfaces: readonly PaletteSurface[];
  jobs: BridgeState["jobs"];
  checkoutRunSheet: BridgeState["checkoutRunSheet"];
  cursor: string | null;
  failing: unknown | null;
  /** Everything a choice can do, in `carryOut`'s own shape. */
  acts: Parameters<typeof carryOut>[2];
  /** A destructive act chosen from the palette, handed on to confirm. */
  onConfirmAct: (id: string, jobId: string) => void;
};

/**
 * **The one surface present whatever else is**, which is why `App.tsx` mounts
 * it as a sibling of the shell rather than inside a screen.
 */
export function PaletteMount({
  open,
  onClose,
  reading,
  shownStudio,
  on,
  surfaces,
  jobs,
  checkoutRunSheet,
  cursor,
  failing,
  acts,
  onConfirmAct,
}: PaletteMountProps) {
  return (
    <Palette
      open={open}
      onClose={onClose}
      // Three places, not two: a Studio open on its whiteboard is neither the
      // Board nor a job read whole, and the acts scoped to it act on the board
      // rather than on anything focused — #1364.
      context={reading !== null ? "detail" : shownStudio === null ? "board" : "studio"}
      // The block is titled with what its rows act on, which on a Studio is the
      // Studio: the acts scoped there put a node on the board.
      on={
        shownStudio !== null && reading === null
          ? `Studio — ${studioName(shownStudio)}`
          : on === undefined
          ? null
          : `${on.id} — ${on.title}`
      }
      surfaces={surfaces}
      filters={reading === null ? BOARD_TABS : []}
      // One row per Check and Command, off the same reading the Manifest
      // surface draws from — Journey 9's own table. Empty until that read has
      // answered, which is what `App.tsx`'s effect holds open.
      runnables={checkoutRunnablesOf(checkoutRunSheet)}
      jobs={jobs.map((job) => ({ id: job.id, label: `${job.handle} — ${job.title}` }))}
      // Fleet settings is the section's first row. It carries no value, because
      // choosing it opens the sheet rather than stating a field.
      settings={[{ id: "fleet_settings", label: "Fleet settings" }]}
      dormant={dormantIn({ reading: reading !== null, cursor, failing: failing !== null })}
      absent={absentIn({ reading: reading !== null, cursor })}
      onChoose={(choice: Parameters<typeof carryOut>[0]) => carryOut(choice, on?.id ?? null, acts)}
      // Every destructive act confirms, even from the palette. It hands the act
      // over and stays open behind the dialog, which is the way back.
      onConfirmAct={(id: string) => {
        if (on?.id !== undefined) onConfirmAct(id, on.id);
      }}
    />
  );
}

export type { BoardTab };
