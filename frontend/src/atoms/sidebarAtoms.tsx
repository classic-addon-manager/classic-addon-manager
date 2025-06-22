import { atom } from 'jotai'
import { type ComponentType } from 'react'
import { Dashboard } from '@/components/Dashboard.tsx'
import { Addons } from '@/components/Addons.tsx'
import { Troubleshooting } from '@/components/Troubleshooting.tsx'

export type PageId = 'dashboard' | 'addons' | 'troubleshooting'

export const activePageAtom = atom<PageId>('dashboard')

const componentMap: Record<PageId, ComponentType> = {
  dashboard: Dashboard,
  addons: Addons,
  troubleshooting: Troubleshooting,
}

export const activeComponentAtom = atom<ComponentType>(get => {
  const activePage = get(activePageAtom)
  return componentMap[activePage]
})

const pageNameMap: Record<PageId, string> = {
  dashboard: 'Dashboard',
  addons: 'Addons',
  troubleshooting: 'Troubleshooting',
}

export const activePageNameAtom = atom<string>(get => {
  const activePage = get(activePageAtom)
  return pageNameMap[activePage]
})
