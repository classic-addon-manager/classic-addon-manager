import { create } from 'zustand'
import { persist } from 'zustand/middleware'

import type { AddonViewMode } from '@/components/addons/types'

interface PreferencesState {
  addonViewMode: AddonViewMode
  setAddonViewMode: (mode: AddonViewMode) => void
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    set => ({
      addonViewMode: 'list',
      setAddonViewMode: mode => set({ addonViewMode: mode }),
    }),
    {
      name: 'ui-preferences',
      partialize: state => ({ addonViewMode: state.addonViewMode }),
    }
  )
)
