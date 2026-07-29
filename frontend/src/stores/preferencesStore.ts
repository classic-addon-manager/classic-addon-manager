import { create } from 'zustand'
import { persist } from 'zustand/middleware'

import type { AddonViewMode } from '@/components/addons/types'
import { type AccentColorId, applyAccentColor, DEFAULT_ACCENT_COLOR } from '@/lib/accentColors'

interface PreferencesState {
  addonViewMode: AddonViewMode
  setAddonViewMode: (mode: AddonViewMode) => void
  accentColor: AccentColorId
  setAccentColor: (id: AccentColorId) => void
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    set => ({
      addonViewMode: 'list',
      setAddonViewMode: mode => set({ addonViewMode: mode }),
      accentColor: DEFAULT_ACCENT_COLOR,
      setAccentColor: id => {
        set({ accentColor: id })
        applyAccentColor(id)
      },
    }),
    {
      name: 'ui-preferences',
      partialize: state => ({
        addonViewMode: state.addonViewMode,
        accentColor: state.accentColor,
      }),
    }
  )
)
