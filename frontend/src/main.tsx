import '@/global.css'

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import App from '@/App'

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
    <App />
  </StrictMode>
)
