import { ReactQueryDevtoolsPanel } from '@tanstack/react-query-devtools'
import { useEffect, useState } from 'react'

export function QueryDevtools() {
  const [open, setOpen] = useState(false)

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        event.code === 'KeyQ' &&
        event.ctrlKey &&
        event.altKey &&
        !event.shiftKey &&
        !event.metaKey &&
        !event.getModifierState('AltGraph') &&
        !event.repeat &&
        !event.isComposing
      ) {
        event.preventDefault()
        setOpen(value => !value)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  if (!open) return null

  return (
    <ReactQueryDevtoolsPanel
      onClose={() => setOpen(false)}
      style={{
        position: 'fixed',
        bottom: 0,
        left: 0,
        width: '100%',
        height: 'min(500px, 80vh)',
        zIndex: 10000,
      }}
    />
  )
}
