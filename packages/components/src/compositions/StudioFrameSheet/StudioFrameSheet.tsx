import { Sheet } from "../../primitives/Sheet/Sheet";
import { STUDIO_FRAME_LABEL, type StudioNodeFrame } from "../StudioNode/StudioNode";

import "./StudioFrameSheet.css";

/**
 * A Note's frame, opened — #1352. On the card it is a screen shrunk to a node's
 * width, which is evidence of nothing; this is where it is read.
 *
 * **A sheet and not a dialog**, because there is nothing to confirm: a sheet is
 * what Bridge opens to read something at length beside the surface it came
 * from, and `widest` is the measure a photograph of a window needs.
 *
 * **What the person said names the sheet.** A sentence under the picture would
 * be somebody's reading of it; what they said at capture is the record.
 */
export type StudioFrameSheetProps = {
  open: boolean;
  /** What the person said at capture — the Note's own words. */
  said: string;
  /** What to draw, or why there is nothing. The caller resolves it, as on the node. */
  frame: StudioNodeFrame;
  onClose?: () => void;
};

export function StudioFrameSheet({ open, said, frame, onClose }: StudioFrameSheetProps) {
  return (
    // Contained, so the layer belongs to the Studios surface rather than
    // covering the shell's rail, which nothing asked it to.
    <Sheet open={open} contained size="widest" title="Note" subtitle={said} onClose={onClose}>
      {frame.src === undefined ? (
        <p className="armada-studio-frame__why">{frame.why ?? "reading…"}</p>
      ) : (
        <img className="armada-studio-frame__image" src={frame.src} alt={STUDIO_FRAME_LABEL} />
      )}
    </Sheet>
  );
}
