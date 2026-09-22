import type { LucideIcon } from "lucide-react";

import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { PathChip } from "../PathChip/PathChip";
import { JOB_STATUS } from "../../generated/vocabulary";

/**
 * What else is running where this work would write.
 *
 * **A fact, never a verdict.** Nothing here greys a control, carries a
 * severity or names a winner — `docs/concepts/fleet.md`, surfaced and never
 * serialised. Dispatching into a file another Job is holding is the ordinary
 * case and this panel exists to let somebody do it knowingly.
 *
 * **Nobody looked and nobody was found are two sentences.** `null` is no
 * comparison having been made, which is every request before its plan claims a
 * path; an empty list is a comparison that ran and found nobody. Drawn alike,
 * the first would say "no overlap" about work nothing has read.
 */
export type WhatElseIsRunningProps = {
  /** One entry per Job, never per path. `null` is nobody having looked. */
  peers: readonly PeerJob[] | null;
  /** The paths the comparison was made against. Empty where none was claimed. */
  pathsAsked?: readonly string[];
  /** Open one of them, where the surface has somewhere to open it. */
  onOpen?: (jobId: string) => void;
};

/** One other Job claiming paths this work claims. */
export type PeerJob = {
  id: string;
  title: string;
  /** Its own status off the wire. `running` and `awaiting_review` are different remedies. */
  status: string;
  /** The narrower of the two claims, per shared path. Never empty. */
  sharedPaths: readonly string[];
};

/** The heading, which is a Wh- panel heading and not a sentence. */
const HEAD = "What else is running";

/** Said under every reading, because it is the one thing this panel must not be mistaken for. */
const NOT_A_VERDICT = "This says what is running. It stops nothing and refuses nothing.";

export function WhatElseIsRunning({ peers, pathsAsked = [], onOpen }: WhatElseIsRunningProps) {
  return (
    <Card className="armada-peers">
      <CardHeader>
        <CardTitle>{HEAD}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="armada-peers__body">
          {peers === null ? (
            <p className="armada-peers__said">
              Nothing has been compared. A request claims no file until its plan names one, so
              there is nothing yet to read another Job against.
            </p>
          ) : peers.length === 0 ? (
            <p className="armada-peers__said">No other Job is writing where this one would.</p>
          ) : (
            <ul className="armada-peers__jobs">
              {peers.map((peer) => (
                <li className="armada-peers__job" key={peer.id}>
                  <div className="armada-peers__head">
                    <span className="armada-peers__title">{peer.title}</span>
                    <At status={peer.status} />
                  </div>
                  {/* The paths the two of them share, which is the whole of
                      what makes this Job worth naming here. */}
                  <div className="armada-peers__paths">
                    {peer.sharedPaths.map((path) => (
                      <PathChip key={path} {...split(path)} title={path} />
                    ))}
                  </div>
                  {onOpen === undefined ? null : (
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => onOpen(peer.id)}
                      aria-label={`Open ${peer.title}`}
                    >
                      Open
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          )}
          {peers === null || pathsAsked.length === 0 ? null : (
            <p className="armada-peers__asked">{`Compared against ${pathsAsked.join(", ")}`}</p>
          )}
          <p className="armada-peers__fact">{NOT_A_VERDICT}</p>
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * A path split for `PathChip`, whose basename never truncates.
 *
 * A path naming a directory keeps its trailing separator on the basename:
 * `crates/api/src/` reads as `crates/api/` and `src/`, so what survives a clip
 * is the segment that says which part of the tree is claimed.
 */
function split(path: string): { directory?: string; basename: string } {
  const directory = path.slice(0, path.replace(/\/$/, "").lastIndexOf("/") + 1);
  const basename = path.slice(directory.length);
  return directory === "" ? { basename } : { directory, basename };
}

/**
 * Where the other Job is, as a badge. **Its own status from the vocabulary**,
 * never a word written here — a second spelling of a status is a second
 * vocabulary.
 */
function At({ status }: { status: string }) {
  const badge = badgeOf(status);
  if (badge === null) return null;
  return (
    <Badge status={badge.status} icon={badge.icon}>
      {badge.verb}
    </Badge>
  );
}

/** `null` where the registry carries no verb, glyph or token — no badge rather than an invented one. */
function badgeOf(status: string): { status: string; icon: LucideIcon; verb: string } | null {
  const rendering = JOB_STATUS[status];
  if (rendering === undefined) return null;
  const { badgeStatus, icon, verb } = rendering;
  if (badgeStatus === null || icon === null || verb === null) return null;
  return { status: badgeStatus, icon, verb };
}
