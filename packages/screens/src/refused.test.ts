// What a stopped Job says it was refused, and the two ways that sentence used
// to be wrong.
//
// **A Job refused nothing must draw nothing**, because most stopped Jobs were
// refused nothing and a heading over an empty list on every one of them says a
// policy was involved where none was.
//
// **A capped list must say it is capped.** A short list read as the whole one
// is worse than no list: a person who widened an allowlist for the fifty they
// could see would think they were done.
//
// **And a list must say what a restart meets.** The rows beside a `Restart
// step` button read as a diagnosis with the button as its cure, and the toolset
// is rendered at spawn — so the press reproduces the refusal it was made from.

import { describe, expect, it } from "vitest";

import type { CommandAnswer, JobDetail as JobWhole, Refusal, Stuck } from "@armada/protocol";
import { refusedIn } from "./refused";

/** One refusal, as the wire carries it: no reason, and the command on `detail`. */
function refusal(over: Partial<Refusal> = {}): Refusal {
  return {
    tool: "Bash",
    call: "toolu_01ANq8Yk3sWnQ2gVv7hHc4Pd",
    detail: "cargo nextest run --package ipc 2>&1 | tail -80",
    truncated: false,
    length: 47,
    because: "",
    ...over,
  };
}

/** A stopped Job, classified, with whatever it was refused. */
function whole(over: Partial<Stuck> = {}): JobWhole {
  return {
    job: {
      id: "01M130Y1380016YK5S0JXBXDQ5",
      handle: "12-a-job",
      title: "Coalesce concurrent token refreshes",
      status: "escalated",
      workflow_id: "bug",
      owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
      origin: "dispatched",
      urgency: "normal",
      atomic: false,
      model: "sonnet",
      created_at: "2026-08-31T09:00:00Z",
    },
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    stuck: {
      stopped_by: "blocked_by_policy",
      step_id: "implement",
      recourse: ["redirect_drone"],
      worktree_on_disk: true,
      drone_unheard: false,
      refused: [],
      refusals: 0,
      ...over,
    },
  };
}

describe("what a stopped job says it was refused", () => {
  it("draws nothing where nothing was refused", () => {
    expect(refusedIn(whole())).toBeUndefined();
  });

  it("draws nothing while the job's own read is in flight", () => {
    expect(refusedIn(null)).toBeUndefined();
  });

  it("draws nothing where fleet classified nothing at all", () => {
    // A Job Fleet serves no `stuck` for is not a Job that was refused nothing —
    // it is one nothing classified — and neither draws a list.
    const unclassified: JobWhole = { ...whole(), stuck: undefined };
    expect(refusedIn(unclassified)).toBeUndefined();
  });

  it("names the command, which is the whole of what was missing", () => {
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 1 }));
    expect(drawn?.refused[0]?.detail).toBe("cargo nextest run --package ipc 2>&1 | tail -80");
    expect(drawn?.refused[0]?.tool).toBe("Bash");
  });

  it("carries no reason where the harness gave none", () => {
    // `because` is empty on every `permission_denied` line observed, and an
    // empty string drawn as a reason is a blank line under every command.
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 1 }));
    expect(drawn?.refused[0]?.because).toBeUndefined();
  });

  it("carries the reason where it gave one", () => {
    const gave = refusal({ because: "rm is not on this drone's allowlist" });
    const drawn = refusedIn(whole({ refused: [gave], refusals: 1 }));
    expect(drawn?.refused[0]?.because).toBe("rm is not on this drone's allowlist");
  });

  it("keeps one row per refusal, never collapsing a repeat", () => {
    // A drone refused the same command five times is a drone that did not
    // learn, which is the most diagnostic thing on the screen.
    const again = [refusal(), refusal(), refusal(), refusal(), refusal()];
    expect(refusedIn(whole({ refused: again, refusals: 5 }))?.refused).toHaveLength(5);
  });

  it("says nothing about a size where the command arrived whole", () => {
    expect(refusedIn(whole({ refused: [refusal()], refusals: 1 }))?.refused[0]?.size).toBeUndefined();
  });

  it("says how much of a cut command is on the row", () => {
    // The failure one level down from the one this branch exists for: a command
    // cut without saying so is one a person pastes into an allowlist whole.
    const cut = refusal({ detail: "cat <<'EOF' > refused.rs", truncated: true, length: 14320 });
    const drawn = refusedIn(whole({ refused: [cut], refusals: 1 }));
    expect(drawn?.refused[0]?.size).toBe("showing 24 of 14,320 characters");
  });

  it("counts a command in the units the wire counted it in", () => {
    // `length` is Rust's `chars().count()`, so a shown count in UTF-16 units
    // would read larger than the total on any command carrying an astral
    // character. Two code points, four UTF-16 units.
    const cut = refusal({ detail: "🚀🚀", truncated: true, length: 900 });
    expect(refusedIn(whole({ refused: [cut], refusals: 1 }))?.refused[0]?.size).toBe(
      "showing 2 of 900 characters",
    );
  });

  it("says a command was cut where nothing recorded how long it was", () => {
    // Never `24 characters shown`, which reads as the whole of it — the
    // sentence that made a cut argument look like a short one.
    const cut = refusal({ detail: "cat <<'EOF' > refused.rs", truncated: true, length: undefined });
    const drawn = refusedIn(whole({ refused: [cut], refusals: 1 }));
    expect(drawn?.refused[0]?.size).toBe("cut at 24 characters");
  });

  it("never draws a cut command as a whole one", () => {
    // The guard, stated over every shape the wire can send rather than over the
    // two above: a truncated refusal always carries a sentence saying so.
    const shapes = [
      refusal({ truncated: true, length: 14320 }),
      refusal({ truncated: true, length: undefined }),
      refusal({ truncated: true, length: 47 }),
      refusal({ detail: "", truncated: true, length: 900 }),
    ];
    const drawn = refusedIn(whole({ refused: shapes, refusals: shapes.length }));
    for (const row of drawn?.refused ?? []) expect(row.size).toBeDefined();
  });

  it("says nothing about a cap where every refusal fits", () => {
    expect(refusedIn(whole({ refused: [refusal()], refusals: 1 }))?.note).toBeUndefined();
  });

  it("says the list is shorter than what happened where it is", () => {
    const drawn = refusedIn(whole({ refused: [refusal(), refusal()], refusals: 137 }));
    expect(drawn?.note).toBe("showing 2 of 137 refused calls");
  });

  it("groups the digits of a count a person has to read", () => {
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 12400 }));
    expect(drawn?.note).toContain("12,400");
  });

  it("never claims fewer refusals than there are rows", () => {
    // Fleet raises a total below what it kept rather than refusing the
    // classification, and this is the same guard from the reading side.
    const drawn = refusedIn(whole({ refused: [refusal(), refusal()], refusals: 1 }));
    expect(drawn?.note).toBeUndefined();
  });

  it("says what would lift the block, on every list there is", () => {
    // The fact that makes the rows actionable, and it is true of every refusal
    // — so no shape of list may arrive without it.
    const shapes = [
      [refusal()],
      [refusal({ tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio" })],
      [refusal({ detail: "" })],
      [refusal(), refusal({ tool: "Write", detail: "xtask/src/rules_layers.rs" })],
    ];
    for (const refused of shapes) {
      const drawn = refusedIn(whole({ refused, refusals: refused.length }));
      expect(drawn?.again).toMatch(/armada\.yml|a restart would be blocked the same way/);
    }
  });

  it("names what declares a command where a command was refused", () => {
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 1 }));
    expect(drawn?.again).toContain("armada.yml");
    expect(drawn?.again).toContain("commands");
    // A command declared and marked destructive is withheld from every drone,
    // so a sentence naming only the section sends a person to add an entry they
    // already have.
    expect(drawn?.again).toContain("destructive");
  });

  it("names no manifest where nothing a manifest declares was refused", () => {
    // `WebFetch` is a capability Armada grants nowhere. Sending a person to
    // `commands` for it is sending them to edit a file that cannot help.
    const web = refusal({ tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio" });
    const drawn = refusedIn(whole({ refused: [web], refusals: 1 }));
    expect(drawn?.again).not.toContain("armada.yml");
  });

  it("names the manifest where one refusal of several is a command", () => {
    // A mixed list still has a command in it, and the sentence is about that
    // row. Withholding it because a `WebFetch` sits beside it would drop the
    // one route on the screen.
    const web = refusal({ tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio" });
    const drawn = refusedIn(whole({ refused: [web, refusal()], refusals: 2 }));
    expect(drawn?.again).toContain("armada.yml");
  });

  it("names the manifest off the tool and never off the command", () => {
    // A `Write` of a path that looks like a shell line is still not a command,
    // and the tool is the wire's own spelling — so the branch reads that and
    // nothing else.
    const wrote = refusal({ tool: "Write", detail: "cargo/config.toml" });
    const drawn = refusedIn(whole({ refused: [wrote], refusals: 1 }));
    expect(drawn?.again).not.toContain("armada.yml");
  });
});

describe("what a person may answer on a refused row", () => {
  const offered: CommandAnswer[] = ["allow_for_job", "always_allow", "reject"];

  it("names the row's own call beside the answers Fleet offered, in its order", () => {
    const drawn = refusedIn(whole({ refused: [refusal({ offers: offered })], refusals: 1 }));
    expect(drawn?.refused[0]?.call).toBe("toolu_01ANq8Yk3sWnQ2gVv7hHc4Pd");
    expect(drawn?.refused[0]?.answers).toEqual([
      { answer: "allow_for_job", label: "Allow for this job" },
      { answer: "always_allow", label: "Always allow in this repository" },
      { answer: "reject", label: "Reject" },
    ]);
  });

  it("draws no answers where Fleet offered none", () => {
    // An older Fleet sends no list, and a job stopped for anything but policy
    // sends an empty one. Both are a row nothing can be done about from here.
    for (const one of [refusal(), refusal({ offers: [] })]) {
      const drawn = refusedIn(whole({ refused: [one], refusals: 1 }));
      expect(drawn?.refused[0]?.answers).toBeUndefined();
    }
  });

  it("leaves out an answer this build has no words for", () => {
    // A Fleet ahead may offer another. A control with no words on it is worse
    // than one control fewer, and the rest of the row still stands.
    const ahead = refusal({ offers: ["allow_once" as CommandAnswer, "reject"] });
    expect(refusedIn(whole({ refused: [ahead], refusals: 1 }))?.refused[0]?.answers).toEqual([
      { answer: "reject", label: "Reject" },
    ]);
  });

  it("says why a row cannot be allowed, in Fleet's own case, as a sentence", () => {
    // Capitalised, the first word would rename a file.
    const kept = refusal({
      detail: "rm -rf .armada",
      offers: [],
      withheld: "armada.yml declares this destructive, as `reset`, and no task runs it unattended",
    });
    expect(refusedIn(whole({ refused: [kept], refusals: 1 }))?.refused[0]?.withheld).toBe(
      "armada.yml declares this destructive, as `reset`, and no task runs it unattended.",
    );
  });

  it("says a command can be allowed from its row where any row offers an answer", () => {
    const drawn = refusedIn(
      whole({ refused: [refusal({ offers: [] }), refusal({ offers: offered })], refusals: 2 }),
    );
    expect(drawn?.again).toBe(
      "A command can be allowed from its row, for this job or for every job in the repository.",
    );
  });

  it("keeps sending a person to armada.yml where no row offers one", () => {
    const drawn = refusedIn(whole({ refused: [refusal({ offers: [] })], refusals: 1 }));
    expect(drawn?.again).toContain("armada.yml");
  });
});
