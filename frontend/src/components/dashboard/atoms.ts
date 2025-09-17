import { atom } from 'jotai'
import { atomWithStore } from 'jotai-zustand'

import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const searchQueryAtom = atom<string>('')
export const selectedAddonAtom = atom<Addon | null>(null)
export const isAddonDialogOpenAtom = atom<boolean>(false)
export const versionSelectAtom = atom<Addon | null>(null)

const addonStoreAtom = atomWithStore(useAddonStore)

export const filteredAddonsAtom = atom(get => {
  const searchQuery = get(searchQueryAtom).toLowerCase()
  const addons = get(addonStoreAtom)
  const { installedAddons } = addons

  if (!searchQuery.trim()) return installedAddons

  return installedAddons.filter(addon => {
    return (
      addon.name.toLowerCase().includes(searchQuery) ||
      addon.alias.toLowerCase().includes(searchQuery) ||
      addon.description?.toLowerCase().includes(searchQuery)
    )
  })
})
