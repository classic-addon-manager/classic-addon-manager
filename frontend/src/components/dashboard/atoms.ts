import { atom } from 'jotai'
import { atomWithStore } from 'jotai-zustand'

import { hasAddonUpdate } from '@/lib/addonUpdate'
import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const searchQueryAtom = atom<string>('')
export const selectedAddonAtom = atom<Addon | null>(null)
export const localUpdateDialogOpenAtom = atom(false)

export const versionSelectAtom = atom<Addon | null>(null)

const addonStoreAtom = atomWithStore(useAddonStore)

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
    const aHasUpdate = hasAddonUpdate(a, latestReleasesMap.get(a.name))
    const bHasUpdate = hasAddonUpdate(b, latestReleasesMap.get(b.name))
    if (aHasUpdate && !bHasUpdate) return -1
    if (!aHasUpdate && bHasUpdate) return 1
    return a.name.localeCompare(b.name)
  })
})
