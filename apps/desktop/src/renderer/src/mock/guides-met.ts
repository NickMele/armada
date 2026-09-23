// Every guide already met, before each browser test.
//
// **A test is not a first-time reader.** A card opening by itself is correct
// behaviour — the owner's own, #1603 — and it is a framed layer with a scrim,
// so in a test about something else it swallows the first press. Eight tests
// across six files went that way the moment the marks landed, each failing on
// a click the card was over.
//
// So the window every other test opens is one whose person has been here
// before. `guides.test.tsx` clears this in its own `beforeEach` to become a
// first-time reader, which is what makes that file the one place the claim is
// made and checked.

import { beforeEach } from "vitest";
import { GUIDES } from "@armada/components";

const KEY = "armada.bridge.guides";

beforeEach(() => {
  window.localStorage.setItem(
    KEY,
    JSON.stringify({ off: false, cardSeen: true, met: GUIDES.map((guide) => guide.piece) }),
  );
});
