import { atom } from 'jotai'
import type { ComponentType } from 'react'

import { Addons } from '@/components/Addons'
import { Dashboard } from '@/components/Dashboard'
import { Settings } from '@/components/Settings'
import { Troubleshooting } from '@/components/Troubleshooting'

export type PageId = 'dashboard' | 'addons' | 'troubleshooting' | 'settings'

export const activePageAtom = atom<PageId>('dashboard')

const componentMap: Record<PageId, ComponentType> = {
  dashboard: Dashboard,
  addons: Addons,
  troubleshooting: Troubleshooting,
  settings: Settings,
}

export const activeComponentAtom = atom<ComponentType>(get => {
  const activePage = get(activePageAtom)
  return componentMap[activePage]
})

const pageNameMap: Record<PageId, string> = {
  dashboard: 'Dashboard',
  addons: 'Addons',
  troubleshooting: 'Troubleshooting',
  settings: 'Settings',
}

export const activePageNameAtom = atom<string>(get => {
  const activePage = get(activePageAtom)
  return pageNameMap[activePage]
})
