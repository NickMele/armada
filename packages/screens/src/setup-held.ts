// Setup, held by the app: the two reads, the ticks, which proposal is open, and what each edit
// and Write came to. Above the screen, which unmounts whenever the rail moves.

import { useEffect, useRef, useState } from "react";
import type { EditManifestProposal, ProposalEdit, WriteManifestProposal } from "@armada/protocol";

import { said } from "./copy";
import type { ManifestProposalsRead, ProposalAnswer, RepositoryScanRead } from "./setup-reads";
import { answered, readInto, type SetupHeld } from "./setup-view";

/** What Setup asks of the host. */
export type SetupSlice = {
  /** Whether Setup is on screen. Nothing is read while it is not. */
  showing: boolean;
  /** The picked repository's root. Another one is a fresh Setup: its ticks and open sheet are not this one's. */
  repository?: string;
  onReadScan: () => Promise<RepositoryScanRead>;
  onReadProposals: () => Promise<ManifestProposalsRead>;
  onEditProposal: (body: EditManifestProposal) => Promise<ProposalAnswer>;
  onWriteProposal: (body: WriteManifestProposal) => Promise<ProposalAnswer>;
};

/** Setup, as the screen draws it and presses it. */
export type Setting = {
  held: SetupHeld;
  onTick: (dir: string, ticked: boolean) => void;
  onOpen: (dir: string) => void;
  onClose: () => void;
  onEdit: (dir: string, edit: ProposalEdit) => void;
  onWrite: (dir: string) => void;
};

export function useSetup(slice: SetupSlice): Setting {
  const [held, setHeld] = useState<SetupHeld>({ state: "reading" });
  const latest = useRef(slice);
  latest.current = slice;

  // Read on every show: another window's edits and a Write are Fleet's, so the picker is
  // re-drawn from them, while ticks and the open proposal stay where the person left them.
  const [heldFor, setHeldFor] = useState(slice.repository);
  if (heldFor !== slice.repository) {
    setHeldFor(slice.repository);
    setHeld({ state: "reading" });
  }

  useEffect(() => {
    if (!slice.showing) return;
    let current = true;
    void Promise.all([latest.current.onReadScan(), latest.current.onReadProposals()]).then(
      ([scan, proposals]) => {
        if (!current) return;
        if (!scan.ok) setHeld({ state: "failed", saying: said(scan.outcome) });
        else if (!proposals.ok) setHeld({ state: "failed", saying: said(proposals.outcome) });
        else setHeld((was) => readInto(was, scan.scan, proposals.proposals));
      },
    );
    return () => {
      current = false;
    };
  }, [slice.showing, slice.repository]);

  const open = (dir: string | null) =>
    setHeld((was) => (was.state === "open" ? { ...was, open: dir } : was));

  function act(dir: string, doing: "edit" | "write", sent: Promise<ProposalAnswer>): void {
    setHeld((was) => (was.state === "open" ? { ...was, busy: dir } : was));
    void sent.then((answer) => setHeld((was) => answered(was, dir, doing, answer, said)));
  }

  return {
    held,
    onTick: (dir, ticked) =>
      setHeld((was) => (was.state === "open" ? { ...was, ticks: { ...was.ticks, [dir]: ticked } } : was)),
    onOpen: open,
    onClose: () => open(null),
    onEdit: (dir, edit) => act(dir, "edit", latest.current.onEditProposal({ dir, edit })),
    onWrite: (dir) => act(dir, "write", latest.current.onWriteProposal({ dir })),
  };
}
