import { create } from 'zustand'

import { ApplicationService } from '@/lib/wails'

interface Settings {
  general?: {
    aacpath?: string
    autodetectpath?: boolean
  }
  [key: string]: unknown
}

interface SettingsState {
  // State
  autoPathDetection: boolean
  aacPath: string
  isInitialized: boolean

  // Actions
  setAutoPathDetection: (enabled: boolean) => void
  setAACPath: (path: string) => void
  loadConfig: () => Promise<void>
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  // Initial state
  autoPathDetection: true,
  aacPath: '',
  isInitialized: false,

  // Actions
  setAutoPathDetection: (enabled: boolean) => {
    set({ autoPathDetection: enabled })
    ApplicationService.SettingsSetAutoDetectPath(enabled)
  },

  setAACPath: (path: string) => {
    set({ aacPath: path })
  },

  loadConfig: async () => {
    const { isInitialized } = get()
    if (!isInitialized) {
      try {
        const config: Settings = await ApplicationService.GetConfig()
        set({
          autoPathDetection: config.general?.autodetectpath ?? true,
          aacPath: config.general?.aacpath ?? '',
          isInitialized: true,
        })
      } catch (error) {
        console.warn('Failed to load default path detection setting:', error)
        set({ isInitialized: true })
      }
    }
  },
}))

// Initialize settings when the store is created
useSettingsStore.getState().loadConfig().catch(console.error)
