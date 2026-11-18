import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// Tauri loads the dev server on a fixed port; 5173 is XkyHold web, 5174 XkyHold-lx.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5175, strictPort: true },
  build: { target: 'chrome120', outDir: 'dist' },
})
