import { atom } from 'jotai'
import { atomWithStore } from 'jotai-zustand'

import type { Addon, Release } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const searchQueryAtom = atom<string>('')
export const selectedAddonAtom = atom<Addon | null>(null)
export const isAddonDialogOpenAtom = atom<boolean>(false)
export const versionSelectAtom = atom<Addon | null>(null)

const addonStoreAtom = atomWithStore(useAddonStore)

const hasUpdate = (addon: Addon, latestReleasesMap: Map<string, Release>) => {
  if (!addon.isManaged) return false
  const release = latestReleasesMap.get(addon.name)
  return release?.published_at > addon.updatedAt
}

export const filteredAddonsAtom = atom(get => {
  const searchQuery = get(searchQueryAtom).toLowerCase()
  const addons = get(addonStoreAtom)
  const { installedAddons, latestReleasesMap } = addons

  // Filter by name, alias, or description when a search query is active
  const filtered = searchQuery.trim()
    ? installedAddons.filter(addon => {
        return (
          addon.name.toLowerCase().includes(searchQuery) ||
          addon.alias.toLowerCase().includes(searchQuery) ||
          addon.description?.toLowerCase().includes(searchQuery)
        )
      })
    : installedAddons

  // Sort addons with available updates to the top, preserve original order otherwise
  return [...filtered].sort((a, b) => {
    const aHasUpdate = hasUpdate(a, latestReleasesMap)
    const bHasUpdate = hasUpdate(b, latestReleasesMap)
    if (aHasUpdate && !bHasUpdate) return -1
    if (!aHasUpdate && bHasUpdate) return 1
    return 0
  })
})
