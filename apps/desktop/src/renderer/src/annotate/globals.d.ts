import type { AnnotationsDevApi } from "../../../shared/annotations";

// The dev-only entry the preload adds when main passed `ANNOTATE_FLAG`, and the
// one Vite value `main.tsx` reads to find a dev server. Spelled as `vite/client`
// spells them, so the two declarations merge if that type is ever added.
declare global {
  interface Window {
    armadaDev?: AnnotationsDevApi;
  }
  interface ImportMetaEnv {
    DEV: boolean;
  }
  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}

export {};
