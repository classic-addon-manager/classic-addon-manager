import { create } from 'zustand'

import { queryClient } from '@/lib/queryClient'
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
  refreshAfterLocalChange: () => Promise<void>

  installWithDependencies: (
    manifest: AddonManifest,
    version: string
  ) => Promise<InstallWithDependenciesResult>
  update: (manifest: AddonManifest, version: string) => Promise<InstallWithDependenciesResult>

  uninstall: (addon: Addon, options?: { skipRefresh?: boolean }) => Promise<boolean>
  unmanage: (addon: Addon) => Promise<boolean>
}

export const useAddonStore = create<AddonState>((set, get) => {
  // Set when an update check is requested while another one is still running.
  let updateCheckQueued = false

  const refreshInstalledAddons = async () => {
    await queryClient.invalidateQueries({ queryKey: ['installed-addons'] })
    try {
      await get().updateInstalledAddons()
    } catch (err) {
      console.error('[AddonStore] Failed to refresh installed addons:', err)
    }
  }

  const refreshUpdateState = async () => {
    await queryClient.invalidateQueries({ queryKey: ['addon-updates-bulk'] })
    try {
      await get().performBulkUpdateCheck()
    } catch (err) {
      console.error('[AddonStore] Failed to perform bulk update check:', err)
    }
  }

  const refreshAfterAddonChange = async () => {
    await refreshInstalledAddons()
    await refreshUpdateState()
  }

  // The update check needs the network, so local changes don't wait for it to finish.
  const refreshAfterLocalChange = async () => {
    await refreshInstalledAddons()
    void refreshUpdateState()
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
    refreshAfterLocalChange,

    performBulkUpdateCheck: async () => {
      const { isCheckingForUpdates, installedAddons } = get()

      if (isCheckingForUpdates) {
        updateCheckQueued = true
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
        const [releases, err] = await safeCall(
          queryClient.fetchQuery({
            queryKey: ['addon-updates-bulk', addonNames],
            queryFn: () => RemoteAddonService.CheckAddonUpdatesBulk(addonNames),
          })
        )

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

        // A newer check is waiting, so these results may include addons that are gone.
        if (!updateCheckQueued) {
          set({ latestReleasesMap, updatesAvailableCount })
        }
      } catch (error) {
        console.error('[AddonStore] Unexpected error caught in performBulkUpdateCheck:', error)
        set({
          latestReleasesMap: new Map<string, Release>(),
          updatesAvailableCount: 0,
        })
      } finally {
        setTimeout(() => {
          set({ isCheckingForUpdates: false })
          if (updateCheckQueued) {
            updateCheckQueued = false
            void get().performBulkUpdateCheck()
          }
        }, 250)
      }
    },

    updateInstalledAddons: async () => {
      const installed = await queryClient.fetchQuery({
        queryKey: ['installed-addons'],
        queryFn: () => LocalAddonService.GetAddOns(),
      })
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

    uninstall: async (addon: Addon, options?: { skipRefresh?: boolean }) => {
      const [result, err] = await safeCall(LocalAddonService.UninstallAddon(addon.name))
      if (err) {
        console.error('[AddonStore] Failed to uninstall addon:', err)
        throw err
      }

      if (!options?.skipRefresh) {
        await refreshAfterLocalChange()
      }

      return result ?? false
    },

    unmanage: async (addon: Addon) => {
      const [result, err] = await safeCall(LocalAddonService.UnmanageAddon(addon.name))
      if (err) {
        console.error('[AddonStore] Failed to unmanage addon:', err)
        throw err
      }

      await refreshAfterLocalChange()

      return result ?? false
    },
  }
})
