import { useAtomValue } from 'jotai'
import type { ReactNode } from 'react'
import { createPortal } from 'react-dom'

import { titleBarSlotAtom } from '@/atoms/titleBarAtoms'

// Renders content straight into the title bar, so toolbar updates never rerender the title bar itself.
export function useTitleBarSlot(content: ReactNode) {
  const slot = useAtomValue(titleBarSlotAtom)
  return slot ? createPortal(content, slot) : null
}
