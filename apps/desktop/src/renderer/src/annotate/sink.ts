// Where the layer's notes are kept: main, over the dev-only preload entry, or a
// vite dev server when the renderer runs in a browser. Both write the same files.

import { ANNOTATIONS_PATH, type Annotation, type AnnotationsDevApi } from "../../../shared/annotations";

export type Sink = AnnotationsDevApi & {
  /** Named in the indicator, so a failed save says which path failed. */
  via: "main" | "dev server";
};

export function mainSink(api: AnnotationsDevApi): Sink {
  return {
    via: "main",
    list: () => api.list(),
    save: (note) => api.save(note),
    remove: (id) => api.remove(id),
    root: () => api.root(),
    capture: (box) => api.capture(box),
  };
}

type Fetch = (url: string, init?: RequestInit) => Promise<Response>;

/**
 * The dev server's routes, `annotations-server.ts`. A `fetch` from the renderer
 * is refused everywhere else in Bridge; this runs only in a browser, where no main exists.
 */
export function devServerSink(fetcher: Fetch = (url, init) => fetch(url, init)): Sink {
  async function ok(response: Response): Promise<Response> {
    if (response.ok) return response;
    throw new Error(
      response.status === 404
        ? `${ANNOTATIONS_PATH} is not served — add annotationsServer() to the dev server's plugins`
        : `${ANNOTATIONS_PATH} answered ${response.status}`,
    );
  }
  const one = (id: string): string => `${ANNOTATIONS_PATH}/${encodeURIComponent(id)}`;
  return {
    via: "dev server",
    list: async () => (await (await ok(await fetcher(ANNOTATIONS_PATH))).json()) as Annotation[],
    save: async (note) => {
      await ok(
        await fetcher(one(note.id), {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(note),
        }),
      );
    },
    remove: async (id) => {
      await ok(await fetcher(one(id), { method: "DELETE" }));
    },
    // A browser has no Fleet to send to and nothing to capture a window with.
    root: async () => null,
    capture: async () => null,
  };
}
