import { atom } from 'jotai'
import type { ComponentType } from 'react'

import { Dashboard } from '@/components/Dashboard'
import { PAGE_DEFINITIONS, type PageId } from '@/components/sidebar/pageDefinitions.ts'

export const activePageAtom = atom<PageId>('dashboard')

export const activeComponentAtom = atom<ComponentType>(get => {
  const activePage = get(activePageAtom)
  const page = PAGE_DEFINITIONS.find(p => p.id === activePage)
  return page?.component ?? Dashboard
})
