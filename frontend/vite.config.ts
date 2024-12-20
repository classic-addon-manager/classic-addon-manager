// @ts-ignore
import path from 'path';
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve("./src/lib"),
      $stores: path.resolve("./src/stores"),
      $screens: path.resolve("./src/screens"),
    },
  }
})
