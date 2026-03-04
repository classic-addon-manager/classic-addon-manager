import { atom } from 'jotai'
import { Blocks, Home, Settings2Icon, WrenchIcon } from 'lucide-react'
import type { ComponentType, ReactNode } from 'react'

import { Addons } from '@/components/Addons'
import { Dashboard } from '@/components/Dashboard'
import { Settings } from '@/components/Settings'
import { Troubleshooting } from '@/components/Troubleshooting'

export const PAGE_DEFINITIONS = [
  {
    id: 'dashboard',
    name: 'Dashboard',
    icon: <Home className="h-4 w-4" />,
    component: Dashboard,
  },
  {
    id: 'addons',
    name: 'Addons',
    icon: <Blocks className="h-4 w-4" />,
    component: Addons,
  },
  {
    id: 'troubleshooting',
    name: 'Troubleshooting',
    icon: <WrenchIcon className="h-4 w-4" />,
    component: Troubleshooting,
  },
  {
    id: 'settings',
    name: 'Settings',
    icon: <Settings2Icon className="h-4 w-4" />,
    component: Settings,
  },
] as const satisfies readonly {
  id: string
  name: string
  icon: ReactNode
  component: ComponentType
}[]

export type PageId = (typeof PAGE_DEFINITIONS)[number]['id']

export const activePageAtom = atom<PageId>('dashboard')

export const activeComponentAtom = atom<ComponentType>(get => {
  const activePage = get(activePageAtom)
  const page = PAGE_DEFINITIONS.find(p => p.id === activePage)
  return page?.component ?? Dashboard
})

export const activePageNameAtom = atom<string>(get => {
  const activePage = get(activePageAtom)
  const page = PAGE_DEFINITIONS.find(p => p.id === activePage)
  return page?.name ?? 'Dashboard'
})
