/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

// Porta fixa: a casca Tauri (src-tauri) aponta para http://localhost:1420.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || false,
    watch: { ignored: ['**/crates/**', '**/src-tauri/**', '**/target/**'] },
  },
  // Os testes de scripts/ci são do node:test (job docs do CI), não do vitest.
  test: { exclude: ['**/node_modules/**', 'scripts/**'] },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
  },
});
