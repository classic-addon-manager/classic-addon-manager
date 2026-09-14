import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react({
      compiler: true,
    }),
    tailwindcss(),
  ],
  resolve: { alias: { '@': import.meta.dirname + '/src' } },
  server: {
    host: '127.0.0.1',
    port: 9245,
    strictPort: true,
  },
})
