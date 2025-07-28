import UI from './UI.tsx'
import { Toaster } from 'sonner'

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
