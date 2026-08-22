import { create } from 'zustand'

import type { AddonViewMode } from '@/components/addons/types'
import { toast } from '@/components/ui/toast'
import {
  ACCENT_PRESETS,
  type AccentColorId,
  applyAccentColor,
  DEFAULT_ACCENT_COLOR,
} from '@/lib/accentColors'
import { safeCall } from '@/lib/utils'
import { ApplicationService } from '@/lib/wails'

const LEGACY_STORAGE_KEY = 'ui-preferences'

const savePreference = async (operation: Promise<unknown>) => {
  const [, err] = await safeCall(operation)
  if (err) {
    toast({
      title: 'Failed to save preferences',
      description: err.message,
    })
  }
}

interface PreferencesState {
  addonViewMode: AddonViewMode
  setAddonViewMode: (mode: AddonViewMode) => void
  accentColor: AccentColorId
  setAccentColor: (id: AccentColorId) => void
  hydrated: boolean
  hydrate: () => Promise<void>
}

const isValidAccentColor = (value: unknown): value is AccentColorId =>
  typeof value === 'string' && ACCENT_PRESETS.some(preset => preset.id === value)

const isValidAddonViewMode = (value: unknown): value is AddonViewMode =>
  value === 'list' || value === 'grid'

const readLegacyPreferences = (): {
  existed: boolean
  accentColor?: AccentColorId
  addonViewMode?: AddonViewMode
} => {
  try {
    const raw = localStorage.getItem(LEGACY_STORAGE_KEY)
    if (raw == null) {
      return { existed: false }
    }

    try {
      const parsed = JSON.parse(raw) as {
        state?: {
          accentColor?: unknown
          addonViewMode?: unknown
        }
      }
      const accentColor = parsed?.state?.accentColor
      const addonViewMode = parsed?.state?.addonViewMode
      return {
        existed: true,
        accentColor: isValidAccentColor(accentColor) ? accentColor : undefined,
        addonViewMode: isValidAddonViewMode(addonViewMode) ? addonViewMode : undefined,
      }
    } catch {
      return { existed: true }
    }
  } catch {
    return { existed: false }
  }
}

export const usePreferencesStore = create<PreferencesState>((set, get) => ({
  addonViewMode: 'list',
  accentColor: DEFAULT_ACCENT_COLOR,
  hydrated: false,

  setAddonViewMode: mode => {
    set({ addonViewMode: mode })
    void savePreference(ApplicationService.SetAddonViewMode(mode))
  },

  setAccentColor: id => {
    set({ accentColor: id })
    applyAccentColor(id)
    void savePreference(ApplicationService.SetAccentColor(id))
  },

  hydrate: async () => {
    const { hydrated } = get()
    if (hydrated) {
      return
    }

    try {
      const [prefs, err] = await safeCall(ApplicationService.GetUIPreferences())
      const legacy = readLegacyPreferences()

      if (err) {
        console.warn('Failed to load UI preferences:', err)
      }

      let accentColor: AccentColorId = DEFAULT_ACCENT_COLOR
      let migratedAccent = false
      if (prefs && isValidAccentColor(prefs.accentColor)) {
        accentColor = prefs.accentColor
      } else if (legacy.accentColor) {
        accentColor = legacy.accentColor
        migratedAccent = true
      }

      let addonViewMode: AddonViewMode = 'list'
      let migratedViewMode = false
      if (prefs && isValidAddonViewMode(prefs.addonViewMode)) {
        addonViewMode = prefs.addonViewMode
      } else if (legacy.addonViewMode) {
        addonViewMode = legacy.addonViewMode
        migratedViewMode = true
      }

      // Only finish the migration when the backend is reachable, so a failed read
      // retries on the next launch instead of dropping the legacy preference.
      if (!err) {
        if (migratedAccent) {
          void savePreference(ApplicationService.SetAccentColor(accentColor))
        }
        if (migratedViewMode) {
          void savePreference(ApplicationService.SetAddonViewMode(addonViewMode))
        }

        if (legacy.existed) {
          try {
            localStorage.removeItem(LEGACY_STORAGE_KEY)
          } catch {
            // ignore
          }
        }
      }

      set({ accentColor, addonViewMode, hydrated: true })
      applyAccentColor(accentColor)
    } catch (error) {
      console.warn('Failed to load UI preferences:', error)
      set({
        accentColor: DEFAULT_ACCENT_COLOR,
        addonViewMode: 'list',
        hydrated: true,
      })
      applyAccentColor(DEFAULT_ACCENT_COLOR)
    }
  },
}))
