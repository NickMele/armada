import { useCallback, useEffect } from "react";
import type { HelmDebugInfo } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { COPY_DEBUG_INFO } from "../../errors/ErrorNotice/payload";
import { copyHelmRecord, helmRecord, HELM_SAFETY } from "./record";

export type { HelmDebugInfo } from "@armada/protocol";
export { copyHelmRecord, helmRecord, HELM_SAFETY } from "./record";

/** The binding, from the contextual key map's `copy debug info` row. */
const BINDING = "c";

/**
 * One Helm session, read before it is copied. #1367.
 *
 * **The error treatment's act on a conversation** — the same word, the same
 * binding, one producer, and an expanded view rendering the string the control
 * copies (`docs/contracts/design-system.md`, *The debug payload*).
 *
 * **A sheet rather than a control in the dock.** The dock is `--w-dock` wide
 * and this is mono text with a brief in it, so a row there could offer the
 * copy and never the reading — and a record that leaves the machine is the one
 * thing a person is made to look at first.
 */
export type HelmRecordProps = {
  open: boolean;
  /** The record, once Fleet has answered. Absent draws what is happening instead. */
  record?: HelmDebugInfo;
  /** Fleet was asked and has not answered yet. */
  reading?: boolean;
  /** Fleet would not answer, in its own words. Drawn instead of the record. */
  failed?: string;
  /** Told after a clipboard write, either way, with the name of what was copied. */
  onCopied?: (what: string) => void;
  onClose?: () => void;
};

export function HelmRecord({ open, record, reading = false, failed, onCopied, onClose }: HelmRecordProps) {
  const copy = useCallback(() => {
    if (record === undefined) return;
    copyHelmRecord(record, onCopied);
  }, [record, onCopied]);

  // `c` runs the control's own function, bound only while this is open: the
  // palette's own `copy debug info` row copies the failure on screen, which is
  // a different subject.
  useEffect(() => {
    if (!open || record === undefined) return;
    function onKey(event: KeyboardEvent): void {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key !== BINDING) return;
      event.preventDefault();
      copy();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, record, copy]);

  const text = record === undefined ? null : helmRecord(record);

  return (
    <Sheet
      open={open}
      size="wide"
      title="Session record"
      // The checkout, not the Manifest id: a person knows their repository by
      // the folder, and the id is in the record a line below.
      subtitle={record === undefined ? undefined : record.checkout}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      footer={
        record === undefined ? undefined : (
          <Button variant="secondary" size="sm" onClick={copy}>
            {COPY_DEBUG_INFO}
          </Button>
        )
      }
    >
      <div className="armada-helm-record">
        {text === null ? (
          <p className="armada-helm-record__note" role="status">
            {failed ?? (reading ? "Reading the session…" : "There is no session to report yet.")}
          </p>
        ) : (
          <>
            <pre className="armada-helm-record__text">{text}</pre>
            <p className="armada-helm-record__safety">{HELM_SAFETY}</p>
          </>
        )}
      </div>
    </Sheet>
  );
}
