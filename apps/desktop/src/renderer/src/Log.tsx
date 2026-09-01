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

import { LogEntry, PayloadLine } from "@armada/components";
import { useState } from "react";

import type { LogRow } from "./story";

export function Log({
  rows,
  emptyNote,
  openId,
  onOpen,
}: {
  rows: LogRow[];
  /** What an empty log says. Never a blank: a blank reads as a failed render. */
  emptyNote: string;
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
    <div className="flex flex-col gap-1" data-armada-log>
      {rows.map((row) => (
        <LogEntry
          key={row.id}
          at={row.at}
          actor={row.actor}
          message={row.message}
          mono={row.mono}
          working={row.working}
          open={open === row.id}
          payloadId={`log-payload-${row.id}`}
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
