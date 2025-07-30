import { create } from 'zustand'

import type { Release } from '@/lib/wails'
import type { Addon } from '@/lib/wails'

interface UpdateDialogState {
  // State
  open: boolean
  addon: Addon | null
  release: Release | null

  // Actions
  setOpen: (open: boolean) => void
  setAddon: (addon: Addon) => void
  setRelease: (release: Release) => void
}

export const useUpdateDialogStore = create<UpdateDialogState>(set => ({
  open: false,
  addon: null,
  release: null,

  setOpen: (open: boolean) => {
    set({ open })
    if (!open) {
      set({ addon: null, release: null })
    }
  },
  setAddon: (addon: Addon) => set({ addon }),
  setRelease: (release: Release) => set({ release }),
}))
