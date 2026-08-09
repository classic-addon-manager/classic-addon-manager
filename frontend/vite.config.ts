import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: [['babel-plugin-react-compiler', {}]],
      },
    }),
    tailwindcss(),
  ],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  server: {
    host: '127.0.0.1',
    port: 9245,
    strictPort: true,
  },
})
