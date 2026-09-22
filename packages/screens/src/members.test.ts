// What the landing-order region draws, read off the members and the landing
// rule: which branch each targets, what finishes the parent, and what
// answering one member's question moves.

import { describe, expect, it } from "vitest";

import { completeWhenSaid, membersOf, movesSaid } from "./members";
import type { LandingRule } from "./draft/landing";
import type { JobMembersView, MemberView } from "./draft/members";

function member(over: Partial<MemberView> & Pick<MemberView, "job">): MemberView {
  return { title: "A member", status: "running", landed: false, ...over };
}

function view(members: MemberView[]): JobMembersView {
  return { job: "parent", title: "Move the settings store out", members };
}

const LANDING: LandingRule = {
  target: "main",
  from_ref: "main",
  prs: "group",
  branching: "group",
  pr_mode: "ready",
  complete_when: "all_members_landed",
  land_together: [],
};

describe("whether the region draws at all", () => {
  it("draws nothing for a Job with no members, which is every Job today", () => {
    expect(membersOf(undefined, LANDING)).toBeUndefined();
    expect(membersOf(view([]), LANDING)).toBeUndefined();
  });

  it("numbers the members from one, in the order they land", () => {
    const props = membersOf(view([member({ job: "a" }), member({ job: "b" })]), LANDING);

    expect(props?.members.map((one) => one.ordinal)).toEqual([1, 2]);
  });
});

describe("which branch a member's pull request targets", () => {
  it("is the branch before it, where it is stacked on that one", () => {
    const props = membersOf(
      view([
        member({ job: "a", branch: "armada/22-one-shape" }),
        member({ job: "b", link: "stacked" }),
      ]),
      LANDING,
    );

    expect(props?.members[1]?.targets).toBe("armada/22-one-shape");
  });

  it("is where the Job lands, for a member that is parked rather than stacked", () => {
    const props = membersOf(
      view([member({ job: "a" }), member({ job: "b", link: "merged" })]),
      LANDING,
    );

    expect(props?.members[1]?.targets).toBe("main");
  });

  it("is where the Job lands for a published member, which is not stacked", () => {
    const props = membersOf(
      view([member({ job: "a" }), member({ job: "b", link: "published" })]),
      LANDING,
    );

    expect(props?.members[1]?.targets).toBe("main");
  });

  it("is nothing where the member before it has no branch yet", () => {
    const props = membersOf(
      view([member({ job: "a" }), member({ job: "b", link: "stacked" })]),
      LANDING,
    );

    expect(props?.members[1]?.targets).toBeUndefined();
  });

  it("is nothing where the Job says nothing about where it lands", () => {
    const props = membersOf(view([member({ job: "a" })]), undefined);

    expect(props?.members[0]?.targets).toBeUndefined();
  });
});

describe("what finishes the parent", () => {
  const three = [
    member({ job: "a", landed: true }),
    member({ job: "b" }),
    member({ job: "c" }),
  ];

  it("counts pull requests merged, not Jobs that reached a successful status", () => {
    const counted = [
      member({ job: "a", landed: true }),
      member({ job: "b", status: "completed_success" }),
    ];

    expect(completeWhenSaid(counted, LANDING)).toContain("1 of 2 pull requests merged");
  });

  it("says every member has to land", () => {
    expect(completeWhenSaid(three, LANDING)).toContain("Done when every member has landed.");
  });

  it("still says so where the Job carries no landing rule at all", () => {
    expect(completeWhenSaid(three, undefined)).toContain("Done when every member has landed.");
  });

  it("names the other three rules where one of them is set", () => {
    expect(completeWhenSaid(three, { ...LANDING, complete_when: "pr_merged" })).toContain(
      "own pull request merges",
    );
    expect(completeWhenSaid(three, { ...LANDING, complete_when: "pr_opened" })).toContain(
      "pull request is open",
    );
    expect(completeWhenSaid(three, { ...LANDING, complete_when: "delivered" })).toContain(
      "delivering step has delivered",
    );
  });

  it("leaves a dropped member out of both halves of the count", () => {
    const with_dropped = [...three, member({ job: "d", dropped: "Folded into the third." })];

    expect(completeWhenSaid(with_dropped, LANDING)).toContain("1 of 3 pull requests merged");
  });

  it("says request in the singular where there is one", () => {
    expect(completeWhenSaid([member({ job: "a" })], LANDING)).toContain(
      "0 of 1 pull request merged",
    );
  });
});

describe("what answering a member's question moves", () => {
  it("names the one pull request waiting behind it", () => {
    expect(movesSaid([member({ job: "c", title: "Drop the store singleton" })])).toBe(
      "1 pull request behind this one is waiting on the answer: Drop the store singleton.",
    );
  });

  it("names every one of several", () => {
    const said = movesSaid([
      member({ job: "c", title: "Drop the singleton" }),
      member({ job: "d", title: "Delete the shim" }),
    ]);

    expect(said).toBe(
      "2 pull requests behind this one are waiting on the answer: Drop the singleton, Delete the shim.",
    );
  });

  it("says so plainly where nothing is behind it", () => {
    expect(movesSaid([])).toBe("Nothing else is waiting on this answer.");
  });

  it("does not count a member somebody already dropped", () => {
    expect(movesSaid([member({ job: "c", title: "Gone", dropped: "Folded in." })])).toBe(
      "Nothing else is waiting on this answer.",
    );
  });
});

describe("a member's own facts", () => {
  it("draws a pull request by its number, never its whole address", () => {
    const props = membersOf(
      view([member({ job: "a", pull_request: "https://git.example/armada/pull/1591" })]),
      LANDING,
    );

    expect(props?.members[0]?.pullRequestLabel).toBe("#1591");
  });

  it("counts tasks without the dropped ones", () => {
    const props = membersOf(
      view([member({ job: "a", tasks: { done: 2, working: 1, open: 1, dropped: 3 } })]),
      LANDING,
    );

    expect(props?.members[0]?.tasks).toBe("2 of 4");
  });

  it("says nothing about a plan with no tasks left in it", () => {
    const props = membersOf(
      view([member({ job: "a", tasks: { done: 0, working: 0, open: 0, dropped: 2 } })]),
      LANDING,
    );

    expect(props?.members[0]?.tasks).toBeUndefined();
  });

  it("renders a status the registry has no row for as the wire spelling", () => {
    const props = membersOf(view([member({ job: "a", status: "reconciling" })]), LANDING);

    expect(props?.members[0]?.state).toEqual({
      as: "text",
      wire: "reconciling",
      missing: "No row in the registry for reconciling",
    });
  });

  it("takes the verb and the glyph from the registry for one it has", () => {
    const props = membersOf(view([member({ job: "a", status: "awaiting_review" })]), LANDING);
    const state = props?.members[0]?.state;

    expect(state?.as).toBe("badge");
    expect(state).toMatchObject({ status: "awaiting-review", label: "awaiting review" });
  });
});

describe("which acts a member offers", () => {
  const acts = { onDropMember: () => undefined };

  it("offers no drop where the caller has nothing to answer it with", () => {
    const props = membersOf(view([member({ job: "a" })]), LANDING);

    expect(props?.members[0]?.onDrop).toBeUndefined();
  });

  it("offers a drop on a member still out", () => {
    const props = membersOf(view([member({ job: "a" })]), LANDING, acts);

    expect(props?.members[0]?.onDrop).toBeDefined();
  });

  it("offers none on one that landed, which has nothing left to close", () => {
    const props = membersOf(view([member({ job: "a", landed: true })]), LANDING, acts);

    expect(props?.members[0]?.onDrop).toBeUndefined();
  });

  it("offers none on one already dropped", () => {
    const props = membersOf(view([member({ job: "a", dropped: "Folded in." })]), LANDING, acts);

    expect(props?.members[0]?.onDrop).toBeUndefined();
  });

  it("draws no question where nothing can answer it", () => {
    const question = {
      step_id: "handoff",
      criterion_id: "c1",
      question: "Does it cover the empty store?",
      expected: "A case",
      produced: "None",
      consequence: "Unproven",
      asked_at: "2026-09-22T10:48:00Z",
    };
    const held = view([member({ job: "a", question })]);

    expect(membersOf(held, LANDING)?.members[0]?.question).toBeUndefined();
    expect(
      membersOf(held, LANDING, { onAnswerJudge: () => undefined })?.members[0]?.question,
    ).toBeDefined();
  });
});
