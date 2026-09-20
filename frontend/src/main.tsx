import '@wailsio/runtime'
import '@/global.css'

import { QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import App from '@/App'
import { QueryDevtools } from '@/components/QueryDevtools'
import { queryClient } from '@/lib/queryClient'

// Load react-scan only in development mode
if (import.meta.env.DEV) {
  import('react-scan')
    .then(reactScan => {
      reactScan.scan()
    })
    .catch(error => {
      console.warn('Failed to load react-scan:', error)
    })
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      {import.meta.env.DEV && <QueryDevtools />}
    </QueryClientProvider>
  </StrictMode>
)
