// Repository-wide always-allow rules — Fleet's own table since protocol 13.5,
// read for the Manifest surface and removable there. #836.
//
// Beside `editing.ts` rather than inside `checkout-runs.ts`: that module
// reads what the Manifest *declares* and runs a rehearsal of it; a
// repository-wide allow is neither — a person granted it from a Job, and it
// binds every job here until somebody removes it. `editing.ts`'s split is the
// model: the wire half and the hook that draws from it, so main can import
// the read type without a component in reach.

import { useEffect, useState } from "react";
import type { Outcome, RepositoryAllowedCommands } from "@armada/protocol";

/**
 * One rule, read for `RunPage.alwaysAllowed`. **Structural, not imported from
 * `@armada/components`** — a component import here would pull every `.tsx`
 * file that package exports into main and preload's project, which type
 * `--jsx`-less on purpose: `RunPageAlwaysAllowedRow` is the same shape, and
 * TypeScript does not need the two names to be one declaration to agree.
 */
export type RepositoryAllowedRow = { run: string };

/**
 * `GET /manifest/allowed_commands` and `POST /manifest/allowed_commands/remove`,
 * read into the app. **One type for both** — `RemoveRepositoryAllowedCommand`'s
 * own reason: removing answers with the same list a read does, so a caller
 * folding one has already folded the other.
 */
export type RepositoryAllowedCommandsRead =
  | { ok: true; commands: RepositoryAllowedCommands }
  | { ok: false; outcome: Outcome };

/** What the Manifest surface asks of the host for this list. */
export type RepositoryAllowsSlice = {
  onListRepositoryAllowedCommands: () => Promise<RepositoryAllowedCommandsRead>;
  onRemoveRepositoryAllowedCommand: (run: string) => Promise<RepositoryAllowedCommandsRead>;
};

/**
 * Every rule a person always-allowed for this repository, read once on
 * opening and again off whatever a removal answers with.
 *
 * **`undefined` until the first read lands.** The rail draws "Reading."
 * rather than an empty list, which would say a person removed every rule
 * there ever was before the read had even gone out.
 */
export function useRepositoryAllows(
  slice: RepositoryAllowsSlice,
): { rows: RepositoryAllowedRow[] | undefined; onRemove: (run: string) => void } {
  const { onListRepositoryAllowedCommands, onRemoveRepositoryAllowedCommand } = slice;
  const [rows, setRows] = useState<RepositoryAllowedRow[] | undefined>(undefined);

  useEffect(() => {
    void onListRepositoryAllowedCommands().then((read) => {
      if (read.ok) setRows(read.commands.commands.map(rowOf));
    });
    // Once, on opening — a removal below folds its own answer rather than
    // waiting for this effect to run again.
  }, []);

  return {
    rows,
    onRemove: (run) => {
      void onRemoveRepositoryAllowedCommand(run).then((read) => {
        if (read.ok) setRows(read.commands.commands.map(rowOf));
      });
    },
  };
}

function rowOf(row: { run: string }): RepositoryAllowedRow {
  return { run: row.run };
}
