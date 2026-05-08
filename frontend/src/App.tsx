import { useEffect } from 'react'
import { Toaster } from 'sonner'

import { useUserStore } from '@/stores/userStore.ts'

import UI from './UI.tsx'

function AuthBootstrap({ children }: { children: React.ReactNode }) {
  const authBootstrapComplete = useUserStore(s => s.authBootstrapComplete)
  const bootstrapAuth = useUserStore(s => s.bootstrapAuth)

  useEffect(() => {
    bootstrapAuth()
  }, [bootstrapAuth])

  if (!authBootstrapComplete) {
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
      <AuthBootstrap>
        <UI />
      </AuthBootstrap>
    </>
  )
}

export default App
