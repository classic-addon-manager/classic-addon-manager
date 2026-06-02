import { useSetAtom } from 'jotai'
import { type ReactNode, useEffect } from 'react'

import { titleBarSlotAtom } from '@/atoms/titleBarAtoms'

export function useTitleBarSlot(content: ReactNode) {
  const setSlot = useSetAtom(titleBarSlotAtom)

  useEffect(() => {
    setSlot(content)
    return () => setSlot(null)
  }, [content, setSlot])
}
