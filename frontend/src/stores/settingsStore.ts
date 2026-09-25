import { create } from 'zustand'

import { toast } from '@/components/ui/toast'
import { safeCall } from '@/lib/utils'
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
  isLoading: boolean
  loadError: string | null

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
  isLoading: false,
  loadError: null,

  // Actions
  setAutoPathDetection: (enabled: boolean) => {
    set({ autoPathDetection: enabled })
    void safeCall(ApplicationService.SettingsSetAutoDetectPath(enabled)).then(([, err]) => {
      if (err) {
        toast({
          title: 'Failed to save settings',
          description: err.message,
        })
      }
    })
  },

  setAACPath: (path: string) => {
    set({ aacPath: path })
  },

  loadConfig: async () => {
    const { isInitialized, isLoading } = get()
    if (isInitialized || isLoading) {
      return
    }
    set({ isLoading: true, loadError: null })
    const [config, err] = await safeCall<Settings>(ApplicationService.GetConfig())
    if (err || !config) {
      console.warn('Failed to load default path detection setting:', err)
      set({ isLoading: false, loadError: err?.message ?? 'Failed to load settings' })
      return
    }
    set({
      autoPathDetection: config.general?.autodetectpath ?? true,
      aacPath: config.general?.aacpath ?? '',
      isInitialized: true,
      isLoading: false,
      loadError: null,
    })
  },
}))
