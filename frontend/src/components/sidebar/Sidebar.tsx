import { Browser } from '@wailsio/runtime'
import { useAtom, useAtomValue } from 'jotai'
import { Globe } from 'lucide-react'

import { versionAtom } from '@/atoms/applicationAtoms'
import { activePageAtom, PAGE_DEFINITIONS } from '@/atoms/sidebarAtoms'
import { UpdateCard } from '@/components/sidebar/UpdateCard'
import { UserBar } from '@/components/sidebar/UserBar'
import { useAddonStore } from '@/stores/addonStore'

import { SidebarItem } from './SidebarItem'

export const Sidebar = () => {
  const [activeItem, setActiveItem] = useAtom(activePageAtom)
  const version = useAtomValue(versionAtom)
  const { updatesAvailableCount } = useAddonStore()

  return (
    <>
      <div className="flex-1 relative">
        <nav className="grid items-start px-2 text-sm font-medium gap-1 relative z-10">
          {PAGE_DEFINITIONS.map(item => (
            <SidebarItem
              key={item.id}
              name={item.name}
              icon={item.icon}
              isActive={activeItem === item.id}
              badgeCount={item.id === 'dashboard' ? updatesAvailableCount : -1}
              onClick={() => setActiveItem(item.id)}
            />
          ))}
          <SidebarItem
            key="website"
            name="AAC Website"
            icon={<Globe className="h-4 w-4" />}
            isActive={false}
            onClick={() => Browser.OpenURL('https://aa-classic.com')}
          />
        </nav>
      </div>

      <UpdateCard />

      <div className="w-full">
        <UserBar />
      </div>
      <div className="mx-auto mb-2 text-muted-foreground opacity-80">
        <span
          className="hover:text-blue-400 cursor-pointer transition-all"
          onClick={() =>
            Browser.OpenURL(
              `https://github.com/classic-addon-manager/classic-addon-manager/releases/tag/v${version}`
            )
          }
        >
          <span>v{version} by Sami</span>
        </span>
      </div>
    </>
  )
}
