// The activity log — chapter two of a step's story, streaming, with nothing in
// front of it.
//
// **It was behind a tab.** The stream only drew after pressing a control called
// *The drone's turns*, inside a four-tab region the drawing has none of. So the
// one chapter that says what is happening right now was the one thing a person
// had to go and find. It is on the page now, at every state, and it fills as
// rows arrive.
//
// **Every line opens in place.** A row and its payload are one thing in the
// order it happened, so a payload opens beneath its row rather than replacing
// the list or opening a pane beside it. Which row is open is held here, because
// it is a property of reading this log and not of the Job.
//
// **The two names below are how the keyboard finds a row, and they are this
// app's own.** `detail-keys.ts` used to reach for `button.armada-entry`, the
// class `LogEntry` happens to ship, so a rename in `packages/components` broke
// `h`/`l` silently. What it reads now is the region attribute this file writes
// and the payload id this file already gave every row — both declared here,
// both read through the helpers, and neither of them a component's internals.

import { Button, LogEntry, PayloadLine } from "@armada/components";
import { useState } from "react";
import type { ReactNode } from "react";

import type { Calls } from "./calls";
import type { CutCall, LogRow } from "./story";
import { sizeOf } from "./story";

/**
 * The attribute on the well that holds a log's rows, carrying which log it is.
 *
 * **A story draws more than one at once** — chapter one's turns and chapter
 * two's preview are two logs over one stream, so the same row is on the screen
 * twice under two ids that are equal. Which one a row belongs to is therefore
 * part of naming it, and this is where that name is written.
 */
export const LOG_REGION = "data-armada-log";

/** What a row's payload is called, so the row's control can point at it. */
export function payloadId(rowId: string): string {
  return `${PAYLOAD}${rowId}`;
}

/**
 * The row a control names, from what its `aria-controls` points at. `null` for
 * anything that is not one of this log's rows — the keyboard reads the whole
 * document and must not act on a control it did not draw.
 */
export function rowOfPayload(names: string | null): string | null {
  if (names === null || !names.startsWith(PAYLOAD)) return null;
  return names.slice(PAYLOAD.length);
}

const PAYLOAD = "log-payload-";

export function Log({
  rows,
  emptyNote,
  region,
  openId,
  onOpen,
  calls,
}: {
  rows: LogRow[];
  /** What an empty log says. Never a blank: a blank reads as a failed render. */
  emptyNote: string;
  /**
   * Which log this is, where the story draws more than one. Two logs over one
   * stream hold the same rows, and a reader who opened a row in chapter one has
   * not opened it in chapter two.
   */
  region: string;
  /** Which row is open, where the caller drives it from the keyboard. */
  openId?: string | null;
  onOpen?: (rowId: string | null) => void;
  /**
   * One Job's fetched call arguments, and how to ask for one. Absent draws a
   * cut row exactly as it arrives — the row still says how much there is, and
   * offers nothing it cannot deliver.
   */
  calls?: Calls;
}) {
  const [held, setHeld] = useState<string | null>(null);
  // The caller's if it has one, and this component's otherwise. A log opened
  // with `l` and a log opened with the pointer are one piece of state, and two
  // would disagree the first time somebody used both.
  const open = openId === undefined ? held : openId;

  if (rows.length === 0) {
    return (
      <p className="text-2xs text-fg-muted" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-1" {...{ [LOG_REGION]: region }}>
      {rows.map((row) => (
        <LogEntry
          key={row.id}
          at={row.at}
          actor={row.actor}
          message={row.message}
          mono={row.mono}
          working={row.working}
          open={open === row.id}
          payloadId={payloadId(row.id)}
          onToggle={() => {
            const next = open === row.id ? null : row.id;
            setHeld(next);
            onOpen?.(next);
          }}
          payload={payloadOf(row, calls)}
          payloadAbsent="This line is the whole of what was recorded."
        />
      ))}
    </div>
  );
}

/**
 * What an open row shows.
 *
 * **The rest of a cut argument lands in this block and nowhere else.** A call
 * whose argument the socket cut opens to what arrived, how much there is, and a
 * control; pressing it replaces those lines with the argument itself, in the
 * same pre block, with its newlines intact. Nothing opens a second surface —
 * the payload is already the place a person went to read this.
 *
 * `undefined` where the row carried nothing, which is what draws `LogEntry`'s
 * own absent line.
 */
function payloadOf(row: LogRow, calls: Calls | undefined): ReactNode {
  const cut = row.call;
  if (cut === undefined || calls === undefined) {
    return row.payload.length === 0 ? undefined : <>{written(row)}</>;
  }
  const held = calls.of(cut.id);
  if (held?.state === "got") {
    // The record's own answer replaces the row's cut line. `whole` is stated
    // rather than inferred, so a record that is itself short still says how
    // much of the argument it has instead of claiming to be all of it.
    const size = held.whole ? undefined : sizeOf(held.arguments.length, held.length);
    return (
      <>
        <PayloadLine>{held.arguments}</PayloadLine>
        {size === undefined ? null : <PayloadLine named="meta">{size}</PayloadLine>}
      </>
    );
  }
  return (
    <>
      {written(row)}
      {held?.state === "absent" ? (
        // An empty state in the shape the rest of the screen uses: one short
        // sentence, and no sentence anywhere naming the transport. The lines
        // above it stay — what arrived is still what was sent.
        <p className="px-3 pt-1 text-2xs text-fg-subtle">{held.note}</p>
      ) : (
        <span className="block px-3 pt-2">
          <Button
            size="sm"
            ground="sunken"
            disabled={held?.state === "fetching"}
            onClick={() => calls.fetch(cut.id)}
          >
            {held?.state === "fetching" ? "Fetching" : labelFor(cut)}
          </Button>
        </span>
      )}
    </>
  );
}

/** The lines the row arrived with. */
function written(row: LogRow): ReactNode {
  return row.payload.map((line, at) => (
    <PayloadLine key={at} named={line.named}>
      {line.text}
    </PayloadLine>
  ));
}

/**
 * What the control offers.
 *
 * **The size is on the line above it, so the label does not repeat it.** A row
 * that carries no size — a transcript written before Fleet stamped one — says
 * nothing about how much there is and still offers the rest, because a size
 * nobody recorded is not a reason to withhold an argument the record has.
 */
function labelFor(cut: CutCall): string {
  return cut.length === undefined ? "Show the whole argument" : "Show the rest";
}
