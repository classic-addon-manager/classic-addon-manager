import { Blocks, Code2, Home, Settings2Icon, WrenchIcon } from 'lucide-react'
import type { ComponentType, SVGProps } from 'react'

import { Addons } from '@/components/Addons.tsx'
import { Dashboard } from '@/components/Dashboard.tsx'
import { Developer } from '@/components/Developer.tsx'
import { Settings } from '@/components/Settings.tsx'
import { Troubleshooting } from '@/components/Troubleshooting.tsx'

export interface PageDefinition {
  id: string
  name: string
  icon: ComponentType<SVGProps<SVGSVGElement>>
  component: ComponentType
}

export const PAGE_DEFINITIONS = [
  {
    id: 'dashboard',
    name: 'Dashboard',
    icon: Home,
    component: Dashboard,
  },
  {
    id: 'addons',
    name: 'Addons',
    icon: Blocks,
    component: Addons,
  },
  {
    id: 'developer',
    name: 'Developer',
    icon: Code2,
    component: Developer,
  },
  {
    id: 'troubleshooting',
    name: 'Troubleshooting',
    icon: WrenchIcon,
    component: Troubleshooting,
  },
  {
    id: 'settings',
    name: 'Settings',
    icon: Settings2Icon,
    component: Settings,
  },
] as const satisfies readonly PageDefinition[]

export type PageId = (typeof PAGE_DEFINITIONS)[number]['id']
