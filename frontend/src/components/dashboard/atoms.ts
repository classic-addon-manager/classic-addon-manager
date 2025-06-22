import { useAddonStore } from '@/stores/addonStore.ts'
import { atomWithStore } from 'jotai-zustand'
import { atom } from 'jotai'

export const searchQueryAtom = atom<string>('')
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
