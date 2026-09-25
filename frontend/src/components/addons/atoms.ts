import { atom } from 'jotai'

import { addonCatalogQuery, fetchAddonCatalog } from '@/lib/catalog'
import { queryClient } from '@/lib/queryClient'
import { daysAgo, NEW_ADDON_DAYS } from '@/lib/utils'
import type { AddonManifest } from '@/lib/wails'
import { LocalAddonService, RemoteAddonService } from '@/lib/wails'

import type { AddonListItem } from './types'

export type { AddonViewMode } from './types'

export const isAddonsReadyAtom = atom(false)
export const searchQueryAtom = atom('')
export const selectedTagAtom = atom('All')
export const isRefreshingAtom = atom(false)

export const selectedManifestAtom = atom<AddonManifest | null>(null)
export const isManifestDialogOpenAtom = atom(false)

export const addonsAtom = atom<AddonListItem[]>([])
export const tagsAtom = atom(['All'])
export const addonsErrorAtom = atom<string | null>(null)

export const loadAddonsAtom = atom(null, async (get, set, force?: boolean) => {
  try {
    if (force) {
      await RemoteAddonService.InvalidateAddonManifestCache()
      await queryClient.invalidateQueries({ queryKey: addonCatalogQuery.queryKey })
    }
    const manifests = await fetchAddonCatalog()

    const installedAddonNames = await LocalAddonService.GetAllInstalledAddonNames()
    const installedAddonSet = new Set(installedAddonNames)

    const tmp: AddonListItem[] = manifests.map(manifest => ({
      manifest,
      isInstalled: installedAddonSet.has(manifest.name),
    }))

    // Generate tags
    const uniqueTags = new Set<string>()
    manifests.forEach(manifest => {
      manifest.tags.forEach(tag => {
        if (tag !== 'Example') {
          uniqueTags.add(tag)
        }
      })
    })

    const sortedTags = Array.from(uniqueTags).sort((a, b) => a.localeCompare(b))
    set(tagsAtom, ['All', ...sortedTags])

    // Sort addons
    tmp.sort((a, b) => {
      const aIsNew = daysAgo(a.manifest.added_at) < NEW_ADDON_DAYS
      const bIsNew = daysAgo(b.manifest.added_at) < NEW_ADDON_DAYS

      if (aIsNew && !bIsNew) return -1
      if (!aIsNew && bIsNew) return 1

      if (aIsNew && bIsNew) {
        const aTime = new Date(a.manifest.added_at).getTime()
        const bTime = new Date(b.manifest.added_at).getTime()
        if (aTime !== bTime) {
          return bTime - aTime
        }
      }

      return a.manifest.name.localeCompare(b.manifest.name)
    })

    set(addonsAtom, tmp)
    set(searchQueryAtom, '')
    set(addonsErrorAtom, null)
    set(isAddonsReadyAtom, true)
    return true
  } catch (err) {
    set(addonsErrorAtom, err instanceof Error ? err.message : String(err))
    set(isAddonsReadyAtom, true)
    return false
  }
})

export const filteredAddonsAtom = atom(get => {
  const addons = get(addonsAtom)
  const selectedTag = get(selectedTagAtom)
  const searchQuery = get(searchQueryAtom).toLowerCase()

  return addons.filter(item => {
    if (selectedTag !== 'All' && !item.manifest.tags.includes(selectedTag)) {
      return false
    }
    return (
      item.manifest.alias.toLowerCase().includes(searchQuery) ||
      item.manifest.description.toLowerCase().includes(searchQuery)
    )
  })
})
