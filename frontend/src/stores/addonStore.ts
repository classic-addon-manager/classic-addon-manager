import { create } from 'zustand'

import { safeCall } from '@/lib/utils.ts'
import type { Addon, AddonManifest, InstallWithDependenciesResult, Release } from '@/lib/wails'
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
  refreshAfterAddonChange: () => Promise<void>

  installWithDependencies: (
    manifest: AddonManifest,
    version: string
  ) => Promise<InstallWithDependenciesResult>
  update: (manifest: AddonManifest, version: string) => Promise<InstallWithDependenciesResult>

  uninstall: (addon: Addon) => Promise<boolean>
  unmanage: (addon: Addon) => Promise<boolean>
}

export const useAddonStore = create<AddonState>((set, get) => {
  const refreshAfterAddonChange = async () => {
    try {
      await get().updateInstalledAddons()
    } catch (err) {
      console.error('[AddonStore] Failed to refresh installed addons:', err)
    }
    try {
      await get().performBulkUpdateCheck()
    } catch (err) {
      console.error('[AddonStore] Failed to perform bulk update check:', err)
    }
  }

  return {
    // Initial state
    installedAddons: [],
    updatesAvailableCount: 0,
    isCheckingForUpdates: false,
    latestReleasesMap: new Map<string, Release>(),

    // Actions
    setAddons: (addons: Array<Addon>) => set({ installedAddons: addons }),

    setUpdatesAvailableCount: (count: number) => set({ updatesAvailableCount: count }),

    refreshAfterAddonChange,

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
            latestReleasesMap: new Map<string, Release>(),
            updatesAvailableCount: 0,
          })
          return
        }

        // Get updates and handle potential errors
        const [releases, err] = await safeCall(RemoteAddonService.CheckAddonUpdatesBulk(addonNames))

        if (err || !releases) {
          console.error('[AddonStore] Failed to perform bulk update check:', err)
          set({
            latestReleasesMap: new Map<string, Release>(),
            updatesAvailableCount: 0,
          })
          return
        }

        // Create map of latest releases and count updates
        const latestReleasesMap = new Map<string, Release>(
          Object.entries(releases).filter(
            (entry): entry is [string, Release] => entry[1] !== undefined
          )
        )

        const updatesAvailableCount = managedAddons.reduce((count, addon) => {
          const latestRelease = latestReleasesMap.get(addon.name)
          if (!latestRelease) return count
          return latestRelease.published_at > addon.updatedAt ? count + 1 : count
        }, 0)

        set({ latestReleasesMap, updatesAvailableCount })
      } catch (error) {
        console.error('[AddonStore] Unexpected error caught in performBulkUpdateCheck:', error)
        set({
          latestReleasesMap: new Map<string, Release>(),
          updatesAvailableCount: 0,
        })
      } finally {
        setTimeout(() => {
          set({ isCheckingForUpdates: false })
        }, 250)
      }
    },

    updateInstalledAddons: async () => {
      const installed = await LocalAddonService.GetAddOns()
      set({ installedAddons: installed })
    },

    installWithDependencies: async (manifest: AddonManifest, version: string) => {
      const [result, err] = await safeCall(
        RemoteAddonService.InstallAddonWithDependencies(manifest, version)
      )
      if (err) {
        console.error('[AddonStore] Failed to install addon with dependencies:', err)
        throw err
      }

      await refreshAfterAddonChange()

      return result as InstallWithDependenciesResult
    },

    update: async (manifest: AddonManifest, version: string) => {
      const [result, err] = await safeCall(RemoteAddonService.UpdateAddon(manifest, version))
      if (err) {
        console.error('[AddonStore] Failed to update addon:', err)
        throw err
      }

      await refreshAfterAddonChange()

      return result as InstallWithDependenciesResult
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
  }
})
