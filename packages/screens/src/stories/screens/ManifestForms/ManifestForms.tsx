// The Manifest surface on its Edit view — the app's own screen, drawn as
// `ManifestFrom` draws it, with the forms in front of the file.

import type { ComponentProps } from "react";

import { ManifestFrom } from "../Manifest/Manifest";

export function ManifestFormsFrom(props: ComponentProps<typeof ManifestFrom>) {
  return <ManifestFrom {...props} view="form" />;
}
