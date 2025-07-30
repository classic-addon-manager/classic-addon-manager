import { useAddonStore } from '@/stores/addonStore'
import { atomWithStore } from 'jotai-zustand'
import { atom } from 'jotai'
import type { Addon } from '@/lib/wails'

export const searchQueryAtom = atom<string>('')
export const selectedAddonAtom = atom<Addon | null>(null)
export const isAddonDialogOpenAtom = atom<boolean>(false)

const addonStoreAtom = atomWithStore(useAddonStore)

export const filteredAddonsAtom = atom(get => {
  const searchQuery = get(searchQueryAtom).toLowerCase()
  const addons = get(addonStoreAtom)
  const { installedAddons } = addons

  return installedAddons.filter(addon => {
    return (
      !searchQuery.trim() ||
      addon.name.toLowerCase().includes(searchQuery) ||
      addon.alias.toLowerCase().includes(searchQuery) ||
      addon.description?.toLowerCase().includes(searchQuery)
    )
  })
})
