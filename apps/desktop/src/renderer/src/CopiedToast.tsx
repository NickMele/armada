// The confirmation a machine value owes.
//
// A machine value copies on click and carries no `copy` glyph — the affordance
// token is the affordance — so **a clipboard write is silent by nature**, and a
// failed one is indistinguishable from a dead element. The contract's answer is
// a toast, and this is the one implementation of it: the root fallback needs
// the same confirmation as the app, and two copies of a timer drift.

import { useEffect, useState } from "react";
import { Toast } from "@armada/components";

/** How long a copy confirmation stands. Long enough to read, short enough to ignore. */
const TOAST_MS = 2400;

export function useCopied(): [string | null, (value: string) => void] {
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    if (copied === null) return undefined;
    const clear = setTimeout(() => setCopied(null), TOAST_MS);
    return () => clearTimeout(clear);
  }, [copied]);

  return [copied, setCopied];
}

/**
 * No dot. The leading dot carries a Job state and is never chosen, and a
 * clipboard write is not a Job state.
 */
export function CopiedToast({ copied }: { copied: string | null }) {
  if (copied === null) return null;
  return (
    <div className="armada-app__toasts">
      <Toast>{`${copied} is on the clipboard.`}</Toast>
    </div>
  );
}
