import { defineConfig } from "vite";

// Tauri sert ce frontend : le port est fixe et connu de tauri.conf.json, et
// l'observateur ignore src-tauri pour ne pas relancer Vite a chaque build Rust.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // La cible est le WebView2 embarque, pas un parc de navigateurs : on peut
    // viser un moteur recent et eviter toute transpilation inutile.
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
