import {
  getInstalledAddons,
  getLatestReleaseMap,
  isCheckingUpdates,
  setCheckingUpdates,
  setInstalledAddons,
  setLatestReleaseMap,
  setUpdateCount,
} from '$atoms/addon.svelte'
import type {AddonManifest} from '$lib/wails'
import {LocalAddonService} from '$lib/wails'
import {RemoteAddonService} from '$lib/wails'

import {safeCall, toast} from './utils'

export default {
  populateAddonStore,
  install,
  uninstall,
  repoHasAddon,
  getManifest,
  unmanage,
  getSubscribedAddons,
  performBulkUpdateCheck,
}

async function getManifest(name: string): Promise<AddonManifest> {
  const manifests = await RemoteAddonService.GetAddonManifest()
  const m = manifests.find((m) => m.name === name)
  if (!m) {
    throw new Error(`Addon not found: ${name}`)
  }
  return m
}

async function repoHasAddon(addon: string): Promise<boolean> {
  const manifests = await RemoteAddonService.GetAddonManifest()
  return manifests.some((m) => m.name === addon)
}

async function install(
  manifest: AddonManifest,
  version: string,
): Promise<boolean> {
  const result = await RemoteAddonService.InstallAddon(manifest, version)
  if (result) {
    await populateAddonStore()
    return true
  }
  return false
}

async function populateAddonStore(): Promise<void> {
  setInstalledAddons(await LocalAddonService.GetAddOns())
}

async function uninstall(addon: string): Promise<boolean> {
  const result = await LocalAddonService.UninstallAddon(addon)
  if (result) {
    await populateAddonStore()
    return true
  }
  return false
}

async function unmanage(addon: string): Promise<boolean> {
  const result = await LocalAddonService.UnmanageAddon(addon)
  if (result) {
    await populateAddonStore()
    return true
  }
  return false
}

async function getSubscribedAddons() {
  const [ads, err] = await safeCall<AddonManifest[]>(
    RemoteAddonService.GetSubscribedAddons(),
  )
  if (err) {
    toast.error('Error getting subscribed addons')
    console.error('Error getting subscribed addons', err)
    return
  }

  if (!ads) {
    return
  }

  let installed = 0

  for (const addon of ads) {
    const [is_installed, err] = await safeCall<boolean>(
      LocalAddonService.IsInstalled(addon.name),
    )
    if (err) {
      console.error('Error checking if addon is installed', err)
      toast.error(
        `Error occurred while checking if addon is installed: ${err}`,
      )
      continue
    }

    if (is_installed) {
      continue
    }

    if (await install(addon, 'latest')) {
      installed++
    }
  }

  if (installed > 0) {
    toast.success('Addons restored', {
      description: `${installed} addons were restored from the server!`,
    })
  }
}

async function performBulkUpdateCheck(): Promise<void> {
  if (isCheckingUpdates()) {
    return
  }

  setCheckingUpdates(true)
  try {
    const managedAddons = getInstalledAddons().filter(
      (addon) => addon.isManaged,
    )
    const addonNames = managedAddons.map((addon) => addon.name)

    // Reset state if no managed addons
    if (addonNames.length === 0) {
      setLatestReleaseMap(new Map())
      setUpdateCount(0)
      return
    }

    // Get updates and handle potential errors
    const [releases, err] = await safeCall(
      RemoteAddonService.CheckAddonUpdatesBulk(addonNames),
    )
    if (err || !releases) {
      console.error('[AddonStore] Failed to perform bulk update check:', err)
      setLatestReleaseMap(new Map())
      setUpdateCount(0)
      return
    }

    // Create map of latest releases and count updates
    setLatestReleaseMap(new Map(Object.entries(releases)))
    setUpdateCount(
      managedAddons.reduce((count, addon) => {
        const latestRelease = getLatestReleaseMap().get(addon.name)
        return latestRelease?.published_at > addon.updatedAt
          ? count + 1
          : count
      }, 0),
    )
  } catch (error) {
    console.error(
      '[AddonStore] Unexpected error caught in performBulkUpdateCheck:',
      error,
    )
    setLatestReleaseMap(new Map())
    setUpdateCount(0)
  } finally {
    setCheckingUpdates(false)
  }
}
