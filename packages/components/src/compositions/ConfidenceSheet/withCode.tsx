import type { ReactNode } from "react";

/** A reviewer writes a name in backticks. Every second piece between them is code. */
export function withCode(text: string): ReactNode {
  return text.split("`").map((piece, at) =>
    at % 2 === 1 ? (
      <code key={at} className="armada-confidence__path">
        {piece}
      </code>
    ) : (
      piece
    ),
  );
}
