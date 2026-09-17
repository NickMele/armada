import { mkdtemp, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import type { IncomingMessage, ServerResponse } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Readable } from "node:stream";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  annotationId,
  isAnnotation,
  isAnnotationId,
  serializeAnnotation,
  type Annotation,
} from "../shared/annotations";
import { listAnnotations, removeAnnotation, repositoryRoot, saveAnnotation } from "./annotations";
import { answer } from "./annotations-server";

function note(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: "20260916-222105-k3f9",
    status: "open",
    text: "The legend is illegible against the row tint",
    component: "JobRowStacked",
    owners: ["ActiveJobsList", "Board"],
    ownersFrom: "parent",
    selector: '#root > div.armada-shell > ul > li:nth-of-type(2)',
    element: { tag: "li", text: "Fix the flaky test", label: null },
    screen: "Job Board",
    layer: null,
    location: "/index.html",
    scenario: null,
    box: { x: 12, y: 80, width: 900, height: 64 },
    window: { width: 1280, height: 800 },
    createdAt: "2026-09-16T22:21:05.000Z",
    updatedAt: "2026-09-16T22:21:05.000Z",
    ...overrides,
  };
}

describe("the note", () => {
  it("takes a sortable id from the time it was written", () => {
    expect(annotationId(new Date("2026-09-16T22:21:05.123Z"), () => 0)).toBe("20260916-222105-0000");
    expect(isAnnotationId(annotationId(new Date(), Math.random))).toBe(true);
  });

  it("refuses an id that could leave the directory", () => {
    for (const id of ["../x", "a/b", "", "a.json", "x".repeat(81)]) expect(isAnnotationId(id)).toBe(false);
  });

  it("is refused with a field missing or mistyped", () => {
    expect(isAnnotation(note())).toBe(true);
    expect(isAnnotation({ ...note(), status: "closed" })).toBe(false);
    expect(isAnnotation({ ...note(), owners: [1] })).toBe(false);
    const { box: _box, ...boxless } = note();
    expect(isAnnotation(boxless)).toBe(false);
  });

  it("serializes with the text and the component first, and parses back", () => {
    const text = serializeAnnotation({ ...note(), updatedAt: "later" });
    expect(Object.keys(JSON.parse(text)).slice(0, 4)).toEqual(["id", "status", "text", "component"]);
    expect(JSON.parse(serializeAnnotation(note()))).toEqual(note());
    expect(text.endsWith("}\n")).toBe(true);
  });
});

describe("the files", () => {
  let dir = "";
  beforeEach(async () => {
    dir = join(await mkdtemp(join(tmpdir(), "annotations-")), ".armada", "annotations");
  });
  afterEach(async () => {
    await rm(join(dir, "..", ".."), { recursive: true, force: true });
  });

  it("writes one file per note, named by its id, and reads them back oldest first", async () => {
    await saveAnnotation(dir, note({ id: "b", createdAt: "2026-09-16T23:00:00.000Z" }));
    await saveAnnotation(dir, note({ id: "a" }));
    expect((await readdir(dir)).sort()).toEqual(["a.json", "b.json"]);
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual(["a", "b"]);
  });

  it("overwrites a note on a second save, and deletes it", async () => {
    await saveAnnotation(dir, note());
    await saveAnnotation(dir, note({ status: "done" }));
    expect((await listAnnotations(dir))[0]?.status).toBe("done");
    await removeAnnotation(dir, note().id);
    expect(await listAnnotations(dir)).toEqual([]);
  });

  it("writes nothing for a value that is not a note", async () => {
    await expect(saveAnnotation(dir, { id: "x" })).rejects.toThrow();
    await expect(removeAnnotation(dir, "../../etc")).rejects.toThrow();
  });

  it("skips a file it cannot read rather than dropping every pin", async () => {
    await saveAnnotation(dir, note());
    await writeFile(join(dir, "broken.json"), "{");
    await writeFile(join(dir, "renamed.json"), serializeAnnotation(note()));
    expect((await listAnnotations(dir)).map((n) => n.id)).toEqual([note().id]);
  });

  it("lists nothing when the directory does not exist yet", async () => {
    expect(await listAnnotations(join(dir, "absent"))).toEqual([]);
  });

  it("finds the repository root above a nested path", async () => {
    const root = join(dir, "..", "..");
    await writeFile(join(root, "pnpm-workspace.yaml"), "");
    await writeFile(join(root, "Cargo.toml"), "");
    await mkdir(join(root, "apps", "desktop"), { recursive: true });
    expect(repositoryRoot(join(root, "apps", "desktop"))).toBe(root);
  });
});

/** A request as connect hands it to the middleware, with the mount path stripped. */
function request(method: string, url: string, body?: string): IncomingMessage {
  const stream = Readable.from(body === undefined ? [] : [Buffer.from(body)]) as unknown as IncomingMessage;
  return Object.assign(stream, { method, url });
}

function response(): ServerResponse & { status: () => number; body: () => string } {
  let status = 0;
  let body = "";
  const res = {
    set statusCode(value: number) {
      status = value;
    },
    setHeader: () => undefined,
    end: (chunk?: string) => {
      body = chunk ?? "";
    },
    status: () => status,
    body: () => body,
  };
  return res as unknown as ServerResponse & { status: () => number; body: () => string };
}

describe("the dev server's routes", () => {
  let dir = "";
  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "annotations-server-"));
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("saves on PUT, lists on GET and deletes on DELETE", async () => {
    const put = response();
    await answer(dir, request("PUT", `/${note().id}`, JSON.stringify(note())), put);
    expect(put.status()).toBe(204);
    expect(JSON.parse(await readFile(join(dir, `${note().id}.json`), "utf8"))).toEqual(note());

    const get = response();
    await answer(dir, request("GET", "/"), get);
    expect(JSON.parse(get.body())).toEqual([note()]);

    const del = response();
    await answer(dir, request("DELETE", `/${note().id}`), del);
    expect(del.status()).toBe(204);
    expect(await readdir(dir)).toEqual([]);
  });

  it("refuses a body whose id is not the path's", async () => {
    const res = response();
    await answer(dir, request("PUT", "/other", JSON.stringify(note())), res);
    expect(res.status()).toBe(400);
    expect(await readdir(dir)).toEqual([]);
  });
});
