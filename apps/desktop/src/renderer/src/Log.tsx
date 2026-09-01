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

import { LogEntry, PayloadLine } from "@armada/components";
import { useState } from "react";

import type { LogRow } from "./story";

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
          payload={
            row.payload.length === 0
              ? undefined
              : row.payload.map((line, at) => (
                  <PayloadLine key={at} named={line.named}>
                    {line.text}
                  </PayloadLine>
                ))
          }
          payloadAbsent="This line is the whole of what was recorded."
        />
      ))}
    </div>
  );
}
