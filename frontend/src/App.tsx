import { useEffect } from 'react'
import { Toaster } from 'sonner'

import { usePreferencesStore } from '@/stores/preferencesStore'
import { useUserStore } from '@/stores/userStore.ts'

import UI from './UI.tsx'

function Bootstrap({ children }: { children: React.ReactNode }) {
  const authBootstrapComplete = useUserStore(s => s.authBootstrapComplete)
  const bootstrapAuth = useUserStore(s => s.bootstrapAuth)
  const hydrated = usePreferencesStore(s => s.hydrated)
  const hydrate = usePreferencesStore(s => s.hydrate)

  useEffect(() => {
    void bootstrapAuth()
    void hydrate()
  }, [bootstrapAuth, hydrate])

  if (!authBootstrapComplete || !hydrated) {
    return null
  }

  return <>{children}</>
}

function App() {
  return (
    <>
      <Toaster
        closeButton
        toastOptions={{
          style: {
            zIndex: 9999,
          },
        }}
      />
      <Bootstrap>
        <UI />
      </Bootstrap>
    </>
  )
}

export default App
