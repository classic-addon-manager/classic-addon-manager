import {svelte} from '@sveltejs/vite-plugin-svelte'
// @ts-ignore
import path from 'path'
import {defineConfig} from 'vite'

// https://vitejs.dev/config/
export default defineConfig(({mode}) => ({
  plugins: [svelte({
    compilerOptions: {}
  })],
  resolve: {
    conditions: ['browser'],
    alias: {
      '@': path.resolve('./src'),
      $lib: path.resolve('./src/lib'),
      $stores: path.resolve('./src/stores'),
      $screens: path.resolve('./src/screens'),
      $atoms: path.resolve('./src/atoms'),
    },
  },
  ssr: {
    noExternal: []
  },
  optimizeDeps: {
    include: ['svelte']
  },
  build: {
    target: 'esnext',
    ssr: false
  },
  define: {
    'import.meta.env.SSR': false
  }
}))
