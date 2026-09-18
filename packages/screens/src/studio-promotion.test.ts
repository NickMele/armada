import { describe, expect, test } from "vitest";
import type { Studio, StudioEdge, StudioNode } from "@armada/protocol";

import { actsOn, aWriteUp, dispatchedAs, placedBeside } from "./studio-promotion";

const at = "2026-09-17T09:00:00.000Z";

function studio(nodes: StudioNode[], edges: StudioEdge[] = []): Studio {
  return {
    id: "01STUDIO",
    manifest_id: "01MANIFEST",
    created_at: at,
    touched_at: at,
    nodes,
    edges,
  };
}

const note = (id: string, said: string, x = 0, y = 0): StudioNode => ({
  id,
  kind: "note",
  said,
  position: { x, y },
  created_at: at,
});

const produced = (from: string, to: string): StudioEdge => ({
  id: `${from}-${to}`,
  from,
  to,
  kind: "produced",
  standing: "accepted",
  created_at: at,
});

describe("what a selection offers", () => {
  test("two Notes cluster; two of anything else only outline", () => {
    const notes = studio([note("a", "first"), note("b", "second")]);
    expect(actsOn(notes, ["a", "b"])).toMatchObject({
      cluster: true,
      outline: true,
      writeUp: false,
    });

    const mixed = studio([
      note("a", "first"),
      {
        id: "b",
        kind: "link",
        address: "https://example.invalid",
        position: { x: 0, y: 0 },
        created_at: at,
      },
    ]);
    expect(actsOn(mixed, ["a", "b"])).toMatchObject({
      cluster: false,
      outline: true,
    });
  });

  test("a Link is read in, not written up", () => {
    const one = studio([
      {
        id: "a",
        kind: "link",
        address: "https://example.invalid",
        position: { x: 0, y: 0 },
        created_at: at,
      },
    ]);
    expect(actsOn(one, ["a"])).toMatchObject({
      writeUp: false,
      defer: true,
      dispatch: false,
      // #1378: the line beside the address is the person's to change.
      editLink: true,
    });
  });

  test("the three forge kinds dispatch, and a Link does not", () => {
    // The kind is Fleet's reading of the address and nothing here reads one:
    // which host is the forge is `crates/adapters`' to know — #1379, #1394.
    const onTheForge = (kind: "issue" | "pull_request" | "epic"): StudioNode => ({
      id: kind,
      kind,
      address: `https://example.invalid/o/r/${kind}/7`,
      number: "7",
      position: { x: 0, y: 0 },
      created_at: at,
    });
    const nodes = studio([
      onTheForge("issue"),
      onTheForge("pull_request"),
      onTheForge("epic"),
      { id: "board", kind: "link", address: "miro://board/uXjVK", position: { x: 0, y: 0 }, created_at: at },
    ]);
    for (const kind of ["issue", "pull_request", "epic"]) {
      expect(actsOn(nodes, [kind])).toMatchObject({ dispatch: true, editLink: true, readIn: true });
    }
    // A Link is an address no adapter recognised, so there is nothing filed to
    // dispatch against — and it is still read in and still takes a line.
    expect(actsOn(nodes, ["board"])).toMatchObject({ dispatch: false, editLink: true, readIn: true });
  });

  test("an Issue draft is the only kind edited or dispatched", () => {
    const draft = studio([
      {
        id: "a",
        kind: "issue_draft",
        title: "A title",
        body: "A body",
        state: "draft",
        position: { x: 0, y: 0 },
        created_at: at,
      },
    ]);
    expect(actsOn(draft, ["a"])).toMatchObject({
      edit: true,
      dispatch: true,
      writeUp: false,
      editLink: false,
    });
  });

  test("a Contradiction ends once: one already ended offers no second outcome", () => {
    const disagreement = (state: string): Studio =>
      studio([
        {
          id: "a",
          kind: "contradiction",
          first: "one",
          second: "two",
          state,
          position: { x: 0, y: 0 },
          created_at: at,
        },
      ]);
    expect(actsOn(disagreement("reported"), ["a"])).toMatchObject({
      settle: true,
      writeUp: true,
      defer: true,
    });
    expect(actsOn(disagreement("resolved_here"), ["a"])).toMatchObject({
      settle: false,
      writeUp: false,
      defer: false,
    });
  });

  test("a selection naming a node the Studio no longer holds is not one node", () => {
    const gone = studio([note("a", "first")]);
    // One of the two is gone, so what is left is one node, and one node's acts are offered.
    expect(actsOn(gone, ["a", "b"])).toMatchObject({ writeUp: true });
    expect(actsOn(gone, ["b"])).toMatchObject({ writeUp: false, defer: false });
  });
});

describe("where a promoted node goes", () => {
  test("to the right of the rightmost it was made from, level with the topmost", () => {
    const two = studio([note("a", "first", 0, 240), note("b", "second", 480, 80)]);
    expect(placedBeside(two, ["a", "b"])).toEqual({ x: 800, y: 80 });
  });

  test("nothing selected is the origin, which no rung reaches", () => {
    expect(placedBeside(studio([]), [])).toEqual({ x: 0, y: 0 });
  });
});

describe("the words a rung opens with", () => {
  test("a write-up of a Cluster carries every Note under it, whole", () => {
    const cluster: StudioNode = {
      id: "c",
      kind: "cluster",
      title: "Counts go stale",
      position: { x: 0, y: 0 },
      created_at: at,
    };
    const graph = studio(
      [note("a", "The chip keeps its count"), note("b", "Overview says three"), cluster],
      [produced("a", "c"), produced("b", "c")],
    );
    const written = aWriteUp(graph, cluster);
    expect(written.title).toBe("Counts go stale");
    expect(written.body).toContain("- The chip keeps its count");
    expect(written.body).toContain("- Overview says three");
  });

  test("a write-up of a Note nothing made is that Note's own words", () => {
    const one = note("a", "The chip keeps its count");
    expect(aWriteUp(studio([one]), one)).toEqual({
      title: "The chip keeps its count",
      body: "The chip keeps its count",
    });
  });

  test("what a dispatch sends is the title, a blank line, the body — and nothing for any other kind", () => {
    const draft: StudioNode = {
      id: "d",
      kind: "issue_draft",
      title: "A title",
      body: "A body",
      state: "draft",
      position: { x: 0, y: 0 },
      created_at: at,
    };
    expect(dispatchedAs(draft)).toBe("A title\n\nA body");
    expect(dispatchedAs(note("a", "said"))).toBeNull();
    // An Issue sends its address alone: the issue it names is already filed.
    const issue: StudioNode = {
      id: "l",
      kind: "issue",
      address: "https://example.invalid/o/r/issues/1379",
      number: "1379",
      title: "An issue on a Studio cannot be dispatched",
      state: "open",
      position: { x: 0, y: 0 },
      created_at: at,
    };
    expect(dispatchedAs(issue)).toBe("https://example.invalid/o/r/issues/1379");
    // A pull request and an epic go the same way — #1394 — and a Link, which
    // is an address no adapter recognised, dispatches nothing.
    expect(dispatchedAs({ ...issue, kind: "pull_request" })).toBe(issue.address);
    expect(dispatchedAs({ ...issue, kind: "epic" })).toBe(issue.address);
    expect(
      dispatchedAs({
        id: "b",
        kind: "link",
        address: "miro://board/uXjVK",
        position: { x: 0, y: 0 },
        created_at: at,
      }),
    ).toBeNull();
  });
});

test("every kind that keeps an address offers Read in, and no other kind does", () => {
  const board = studio([
    { id: "link", kind: "link", address: "miro://board/uXjVK", position: { x: 0, y: 0 }, created_at: at },
    {
      id: "epic",
      kind: "epic",
      address: "https://example.invalid/o/r/milestone/17",
      number: "17",
      position: { x: 0, y: 400 },
      created_at: at,
    },
    { id: "note", kind: "note", said: "the rail is unreadable", position: { x: 0, y: 200 }, created_at: at },
  ]);
  // **Offered on every node with an address, whatever it is.** Which addresses
  // are sources is Fleet's, and a rule copied here would drift from it — and
  // the Link somebody pasted and the Epic it turned out to be are one node.
  expect(actsOn(board, ["link"]).readIn).toBe(true);
  expect(actsOn(board, ["epic"]).readIn).toBe(true);
  expect(actsOn(board, ["epic"]).editLink).toBe(true);
  expect(actsOn(board, ["note"]).readIn).toBe(false);
  expect(actsOn(board, ["link", "note"]).readIn).toBe(false);
});
