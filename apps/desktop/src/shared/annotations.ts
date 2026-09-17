// A note left on Bridge's own UI, in development, for a coding agent to act on.
// #1226.
//
// **Not on `window.armada`.** That surface is Fleet's seam, and the fake of it
// that runs the renderer in a browser (#1223) must not have to answer this. The
// layer reaches its files through `window.armadaDev` inside Electron, and over
// HTTP from a vite dev server in a browser — both write the same files, in the
// shape below.
//
// Nothing here touches the DOM or Node, because main, the preload, the renderer
// and a vite plugin all read it.

/** Where the notes land, from the repository root. One JSON file per note. */
export const ANNOTATIONS_DIR = [".armada", "annotations"] as const;

/** The path a vite dev server answers on, for the renderer in a browser. */
export const ANNOTATIONS_PATH = "/__armada/annotations";

/**
 * The argument main passes the window when the app is not packaged. The
 * preload exposes `armadaDev` only when it sees it, so a packaged build has no
 * handler, no preload entry and no reason to load the layer's chunk.
 */
export const ANNOTATE_FLAG = "--armada-annotate";

export const ANNOTATION_CHANNELS = {
  list: "annotations:list",
  save: "annotations:save",
  remove: "annotations:remove",
} as const;

export type AnnotationStatus = "open" | "done";

export type Box = { x: number; y: number; width: number; height: number };

/**
 * One note. Written for the agent that reads it, so each field is something a
 * search for the code can start from.
 */
export type Annotation = {
  /** Also the file name, less `.json`. Sorts by when it was written. */
  id: string;
  status: AnnotationStatus;
  /** What the person said. */
  text: string;
  /** The innermost React component under the click, or null if none was found. */
  component: string | null;
  /** The components above it, nearest first. */
  owners: string[];
  /**
   * Which chain `owners` is. `owner` is who rendered it, which React keeps only
   * in a development build; `parent` is the tree it sits in, which every build
   * has. `pnpm dev` runs a production React, so Bridge under it says `parent`.
   */
  ownersFrom: "owner" | "parent";
  /** A CSS selector the layer found the element by, and finds it by again. */
  selector: string;
  element: { tag: string; text: string; label: string | null };
  /** The rail item marked current when the note was written, or null. */
  screen: string | null;
  /** The open dialog or sheet the element sits in, by its accessible name. */
  layer: string | null;
  /** Path, query and hash of the page. */
  location: string;
  /** `?scenario=` in the mock (#1223), or null. */
  scenario: string | null;
  /** The element's box in the window, in CSS pixels. */
  box: Box;
  window: { width: number; height: number };
  createdAt: string;
  updatedAt: string;
};

/** What `window.armadaDev` carries. Three operations, each on one note. */
export type AnnotationsDevApi = {
  list: () => Promise<Annotation[]>;
  save: (note: Annotation) => Promise<void>;
  remove: (id: string) => Promise<void>;
};

/** An id is a file name, so it is held to what cannot leave the directory. */
export function isAnnotationId(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{1,80}$/.test(value);
}

/** `20260916-222105-k3f9`: sortable, readable in `ls`, and unique enough by hand. */
export function annotationId(at: Date, random: () => number = Math.random): string {
  const iso = at.toISOString();
  const stamp = `${iso.slice(0, 10).replaceAll("-", "")}-${iso.slice(11, 19).replaceAll(":", "")}`;
  const suffix = Math.floor(random() * 36 ** 4)
    .toString(36)
    .padStart(4, "0");
  return `${stamp}-${suffix}`;
}

const isString = (v: unknown): v is string => typeof v === "string";
const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isNullableString = (v: unknown): v is string | null => v === null || isString(v);
const isRecord = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);

function isBox(v: unknown): v is Box {
  return isRecord(v) && isNumber(v["x"]) && isNumber(v["y"]) && isNumber(v["width"]) && isNumber(v["height"]);
}

/**
 * Whether a value read off the wire or the disk is a note. Main and the vite
 * plugin both refuse anything else, so a malformed body writes nothing.
 */
export function isAnnotation(value: unknown): value is Annotation {
  if (!isRecord(value)) return false;
  const element = value["element"];
  const window = value["window"];
  return (
    isAnnotationId(value["id"]) &&
    (value["status"] === "open" || value["status"] === "done") &&
    isString(value["text"]) &&
    isNullableString(value["component"]) &&
    Array.isArray(value["owners"]) &&
    value["owners"].every(isString) &&
    (value["ownersFrom"] === "owner" || value["ownersFrom"] === "parent") &&
    isString(value["selector"]) &&
    isRecord(element) &&
    isString(element["tag"]) &&
    isString(element["text"]) &&
    isNullableString(element["label"]) &&
    isNullableString(value["screen"]) &&
    isNullableString(value["layer"]) &&
    isString(value["location"]) &&
    isNullableString(value["scenario"]) &&
    isBox(value["box"]) &&
    isRecord(window) &&
    isNumber(window["width"]) &&
    isNumber(window["height"]) &&
    isString(value["createdAt"]) &&
    isString(value["updatedAt"])
  );
}

/**
 * The file's contents. Keys in the order the type declares them, so the first
 * lines an agent reads are the note and where it points.
 */
export function serializeAnnotation(note: Annotation): string {
  const ordered: Annotation = {
    id: note.id,
    status: note.status,
    text: note.text,
    component: note.component,
    owners: note.owners,
    ownersFrom: note.ownersFrom,
    selector: note.selector,
    element: note.element,
    screen: note.screen,
    layer: note.layer,
    location: note.location,
    scenario: note.scenario,
    box: note.box,
    window: note.window,
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
  };
  return `${JSON.stringify(ordered, null, 2)}\n`;
}

/** Oldest first, which is the order they were written in and the pins' numbers. */
export function byCreation(a: Annotation, b: Annotation): number {
  return a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : a.id < b.id ? -1 : 1;
}
