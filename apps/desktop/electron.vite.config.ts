import { defineConfig } from 'electron-vite'

// Three builds from one config: the main process, the preload bridge, and the
// renderer. The preload is the only thing that crosses between them, which is
// why it is a build of its own rather than an import.
export default defineConfig({
  main: { build: { lib: { entry: 'src/main/index.ts' } } },
  preload: { build: { lib: { entry: 'src/preload/index.ts' } } },
  renderer: { root: 'src/renderer' },
})
