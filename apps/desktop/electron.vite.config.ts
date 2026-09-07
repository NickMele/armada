import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import type { Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'

// The app tile, emitted beside the main bundle — twice, in two formats, for
// two different readers.
//
// `packages/brand/macos/AppIcon.icns` is the built icon and its README says it
// is ready to drop into a bundle. Nothing was dropping it, so the running app
// and its dock tile carried Electron's own mark. That was fixed by emitting it
// — and the tile stayed Electron's anyway, for a second reason nothing here
// could see.
//
// **`nativeImage.createFromPath` does not read `.icns`.** Measured against
// Electron 40.10.6 on 2026-09-07: the call returns an empty image and reports
// 0×0, for the checked-in `.icns` and for one rebuilt with `iconutil` from the
// same iconset, while the iconset's own PNG at the same path style loads at
// 512×512. The file was never the problem and rebuilding it never would have
// helped. So the runtime tile is a PNG, and `src/main/index.ts` reads that one.
//
// **The `.icns` is still emitted, because it is what a packager wants.** There
// is no packager in this workspace yet; when there is, its `icon` key takes the
// `.icns` and macOS reads it through `Info.plist`, which is a different code
// path from `nativeImage` and does handle the format. Dropping it now would
// mean putting it back then, having lost the reason.
//
// Both are emitted rather than imported because the main process is a Node
// bundle and needs a path on disk, not a module — so the build writes them next
// to `index.js` and `src/main/index.ts` resolves them from `__dirname`. That
// works the same in `dev`, `build` and `preview`, which a path reaching back
// into `packages/` through a workspace symlink would not.
//
// **The accent-filled variant is what these carry, and the app tile is the only
// place it is allowed.** `packages/icons/icons.toml` states it on the
// `armada-mark` row: "Accent fill permitted on the macOS app tile alone."
const APP_ICONS = ['AppIcon.icns', 'AppIcon.png'] as const

function appIcon(): Plugin {
  const require = createRequire(import.meta.url)
  return {
    name: 'armada-app-icon',
    generateBundle() {
      for (const fileName of APP_ICONS) {
        // Resolved through the package's own `exports`, so `packages/brand`
        // says the tile is part of its surface rather than this build reaching
        // into its directory layout.
        const source = readFileSync(require.resolve(`@armada/brand/${fileName}`))
        this.emitFile({ type: 'asset', fileName, source })
      }
    },
  }
}

/* Every workspace package, read off this app's own dependencies rather than
   listed here. A package added to `package.json` is bundled from the day it is
   added, with nothing to keep in step. */
const WORKSPACE = Object.keys(
  JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')).dependencies,
).filter((name) => name.startsWith('@armada/'))

// Three builds from one config: the main process, the preload bridge, and the
// renderer. The preload is the only thing that crosses between them, which is
// why it is a build of its own rather than an import.
export default defineConfig({
  // Dependencies stay external in the main bundle: `ws` reaches for node
  // builtins and optional native helpers, and bundling it would inline
  // decisions its own package makes at require time. The token set is the
  // exception — it is data this build reads, and inlining it means no runtime
  // resolution of a JSON file through a workspace symlink.
  main: {
    // **`no-external` is what keeps `lucide-react` out of a Node bundle.** Main
    // reads the Board's needs-you rule so it can notify about a job that has
    // started waiting, and that rule reads the generated vocabulary — one module
    // that also carries the glyph for every status. Rollup keeps a bare
    // `require` for an external whose bindings it dropped unless it is told the
    // module has no side effects, so without this line the main process loads an
    // icon library it never calls. Measured: the require is there with the
    // default and gone with this.
    build: {
      lib: { entry: 'src/main/index.ts' },
      rollupOptions: { treeshake: { moduleSideEffects: 'no-external' } },
    },
    // **No workspace package is external.** `externalizeDepsPlugin` leaves a
    // dependency for Node to resolve at runtime, which is right for something
    // published as JavaScript and wrong for every `@armada/*`: they are
    // TypeScript source behind a symlink, so Node reaches `src/index.ts` and
    // fails on the first extensionless import.
    //
    // It cost a broken `pnpm dev` to learn where the seam is. `bridge_build`
    // was green throughout, because building the main bundle never resolves
    // what it externalised — only running it does.
    plugins: [externalizeDepsPlugin({ exclude: WORKSPACE }), appIcon()],
  },
  // The preload externalises by default too, and gets the same treatment for
  // the same reason: it imports the wire's channel names, and a `require` for
  // a workspace package fails in the preload exactly as it does in main.
  preload: {
    plugins: [externalizeDepsPlugin({ exclude: WORKSPACE })],
    build: { lib: { entry: 'src/preload/index.ts' } },
  },
  // Tailwind v4 has no JS config: the theme is packages/tokens/tokens.theme.css,
  // generated by `cargo xtask verify-tokens` and imported by the renderer's
  // stylesheet. There is deliberately no tailwind.config.js to drift from it.
  renderer: { root: 'src/renderer', plugins: [react(), tailwindcss()] },
})
