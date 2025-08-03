import { atom } from 'jotai'

import { daysAgo } from '@/lib/utils'
import { LocalAddonService, RemoteAddonService } from '@/lib/wails'

import type { AddonListItem } from './types'

export const isAddonsReadyAtom = atom(false)
export const searchQueryAtom = atom('')
export const selectedTagAtom = atom('All')

export const addonsAtom = atom<AddonListItem[]>([])
export const tagsAtom = atom(['All'])

export const loadAddonsAtom = atom(null, async (get, set) => {
  const manifests = await RemoteAddonService.GetAddonManifest()
  // Future todo: we could improve performance by getting a list of all installed addons rather than checking in a loop
  const installStatusPromises = manifests.map(m => LocalAddonService.IsInstalled(m.name))
  const installStatuses = await Promise.all(installStatusPromises)

  const tmp: AddonListItem[] = manifests.map((manifest, index) => ({
    manifest,
    isInstalled: installStatuses[index],
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
    const aIsNew = daysAgo(a.manifest.added_at) < 32
    const bIsNew = daysAgo(b.manifest.added_at) < 32

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
  set(isAddonsReadyAtom, true)
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
