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

/**
 * The same timer, for a sentence the app already wrote.
 *
 * **Why a second hook rather than a second argument to the first.** `useCopied`
 * holds a *value* and the toast composes the sentence around it, so a caller
 * cannot say anything else through it. Opening a file has five ways to fail and
 * each is its own sentence — `renderer/src/opening.ts` writes them — and
 * "`.armada/briefs/…` is on the clipboard" is not one of them.
 */
export function useSaid(): [string | null, (sentence: string) => void] {
  const [said, setSaid] = useState<string | null>(null);

  useEffect(() => {
    if (said === null) return undefined;
    const clear = setTimeout(() => setSaid(null), TOAST_MS);
    return () => clearTimeout(clear);
  }, [said]);

  return [said, setSaid];
}

/**
 * A sentence the app is telling somebody, carried whole.
 *
 * **An open that did nothing is the defect being fixed**, so a click that ends
 * in nothing on screen is not an outcome this surface may have. Success says
 * nothing — the file is in front of them, in their editor, which says it better
 * than a toast would.
 */
export function SaidToast({ said }: { said: string | null }) {
  if (said === null) return null;
  return (
    <div className="armada-app__toasts">
      <Toast>{said}</Toast>
    </div>
  );
}
