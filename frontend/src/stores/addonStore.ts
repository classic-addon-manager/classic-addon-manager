import { create } from 'zustand'

import { safeCall } from '@/lib/utils.ts'
import type { Addon, AddonManifest, Release } from '@/lib/wails'
import { LocalAddonService, RemoteAddonService } from '@/lib/wails'

interface AddonState {
  // State
  installedAddons: Array<Addon>
  updatesAvailableCount: number
  isCheckingForUpdates: boolean
  latestReleasesMap: Map<string, Release>

  // Actions
  setAddons: (addons: Array<Addon>) => void
  setUpdatesAvailableCount: (count: number) => void
  performBulkUpdateCheck: () => Promise<void>
  updateInstalledAddons: () => Promise<void>

  install: (manifest: AddonManifest, version: string) => Promise<boolean>

  uninstall: (addon: Addon) => Promise<boolean>
  unmanage: (addon: Addon) => Promise<boolean>
}

export const useAddonStore = create<AddonState>((set, get) => ({
  // Initial state
  installedAddons: [],
  updatesAvailableCount: 0,
  isCheckingForUpdates: false,
  latestReleasesMap: new Map<string, Release>(),

  // Actions
  setAddons: (addons: Array<Addon>) => set({ installedAddons: addons }),

  setUpdatesAvailableCount: (count: number) => set({ updatesAvailableCount: count }),

  performBulkUpdateCheck: async () => {
    const { isCheckingForUpdates, installedAddons } = get()

    if (isCheckingForUpdates) {
      return
    }

    set({ isCheckingForUpdates: true })

    try {
      const managedAddons = installedAddons.filter(addon => addon.isManaged)
      const addonNames = managedAddons.map(addon => addon.name)

      // Reset state if no managed addons
      if (addonNames.length === 0) {
        set({
          latestReleasesMap: new Map(),
          updatesAvailableCount: 0,
        })
        return
      }

      // Get updates and handle potential errors
      const [releases, err] = await safeCall(RemoteAddonService.CheckAddonUpdatesBulk(addonNames))

      if (err || !releases) {
        console.error('[AddonStore] Failed to perform bulk update check:', err)
        set({
          latestReleasesMap: new Map(),
          updatesAvailableCount: 0,
        })
        return
      }

      // Create map of latest releases and count updates
      const latestReleasesMap = new Map(Object.entries(releases))

      const updatesAvailableCount = managedAddons.reduce((count, addon) => {
        const latestRelease = latestReleasesMap.get(addon.name)
        return latestRelease?.published_at > addon.updatedAt ? count + 1 : count
      }, 0)

      set({ latestReleasesMap, updatesAvailableCount })
    } catch (error) {
      console.error('[AddonStore] Unexpected error caught in performBulkUpdateCheck:', error)
      set({
        latestReleasesMap: new Map(),
        updatesAvailableCount: 0,
      })
    } finally {
      set({ isCheckingForUpdates: false })
    }
  },

  updateInstalledAddons: async () => {
    const installed = await LocalAddonService.GetAddOns()
    set({ installedAddons: installed })
  },

  install: async (manifest: AddonManifest, version: string) => {
    const [result, err] = await safeCall(RemoteAddonService.InstallAddon(manifest, version))
    if (err) {
      console.error('[AddonStore] Failed to install addon:', err)
      return false
    }

    if (result) {
      await get().updateInstalledAddons()
      return true
    }

    return false
  },

  uninstall: async (addon: Addon) => {
    const [result, err] = await safeCall(LocalAddonService.UninstallAddon(addon.name))
    if (err) {
      console.error('[AddonStore] Failed to uninstall addon:', err)
      throw err
    }

    await get().updateInstalledAddons()

    return result ?? false
  },

  unmanage: async (addon: Addon) => {
    const [result, err] = await safeCall(LocalAddonService.UnmanageAddon(addon.name))
    if (err) {
      console.error('[AddonStore] Failed to unmanage addon:', err)
      throw err
    }

    await get().updateInstalledAddons()

    return result ?? false
  },
}))
