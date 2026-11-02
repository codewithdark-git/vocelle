import { defineConfig } from "vite";

// Tauri expects a fixed dev port and serves the built files from dist/.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    // The app's content security policy only allows files it ships, not
    // data: URLs, so small assets like the logo are emitted as files too.
    assetsInlineLimit: 0,
    rollupOptions: {
      // One page per Tauri window.
      input: {
        main: "index.html",
        overlay: "overlay.html",
      },
    },
  },
});
