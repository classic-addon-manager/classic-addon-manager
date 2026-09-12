import { atom } from 'jotai'
import { createElement } from 'react'

import { Dashboard } from '@/components/Dashboard'
import { PAGE_DEFINITIONS, type PageId } from '@/components/sidebar/pageDefinitions.ts'

export const activePageAtom = atom<PageId>('dashboard')

export const activePageElementAtom = atom(get => {
  const activePage = get(activePageAtom)
  const page = PAGE_DEFINITIONS.find(p => p.id === activePage)
  return createElement(page?.component ?? Dashboard)
})
