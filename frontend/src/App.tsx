import { Toaster } from 'sonner'

import UI from './UI.tsx'

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
      <UI />
    </>
  )
}

export default App
