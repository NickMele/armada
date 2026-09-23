// What a moment has already typed, before Fleet can answer any of it.
//
// **The draft schema has no read behind it** (#1532), so a board built in this
// milestone cannot get its values off the wire. A mock scenario carries them
// on itself and hands them down here; the app's own mount provides nothing, so
// every field is absent and each surface draws what it draws against a real
// Fleet. That is the seam this context is, and it is the whole of it.
//
// It moves onto a read the day a shape is promoted (#1545), and this file goes.

import { createContext, useContext } from "react";
import type { ReactNode } from "react";

import type { BranchesAnswer } from "@armada/screens/src/draft/branches";
import type { LandingRule } from "@armada/screens/src/draft/landing";
import type { ProposalView } from "@armada/screens/src/draft/proposal";
import type { SketchAttachment } from "@armada/screens/src/draft/sketch";

/** What a moment holds for the surface that dispatches. Every field optional. */
export type Drafted = {
  /** What is already in the request field. Absent opens it empty. */
  prompt?: string;
  /** Where the work starts and where it lands. Absent falls back to the Manifest. */
  landing?: LandingRule;
  /**
   * The repository's branches, for the two ref fields to pick over. Absent is
   * nothing having listed them, which is the app on a real Fleet.
   */
  branches?: BranchesAnswer;
  /**
   * The proposal this moment is of — the tier map, the two caps, the refs.
   * **The dispatch form and the classifying screen read the same shape**, one
   * moment apart, which is why this is not a settings type of its own.
   */
  proposal?: ProposalView;
  /**
   * The picture beside the prompt. **Absent opens Sketch on a blank pad**,
   * which is every dispatch somebody starts in the app.
   */
  sketch?: SketchAttachment;
};

/** Nothing drafted, which is the app on a real Fleet. */
const NOTHING: Drafted = {};

const Held = createContext<Drafted>(NOTHING);

/** What this window was mounted holding. `{}` where nobody provided any. */
export function useDrafted(): Drafted {
  return useContext(Held);
}

/** Hand a moment's draft to the window. The mock's own mount is the one caller. */
export function DraftedFrom({ held, children }: { held: Drafted; children: ReactNode }) {
  return <Held.Provider value={held}>{children}</Held.Provider>;
}
