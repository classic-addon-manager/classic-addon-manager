import { Browser } from '@wailsio/runtime'
import { useAtom, useAtomValue } from 'jotai'
import { ArrowUpCircle, Globe } from 'lucide-react'

import { updateAvailableAtom, updateDialogOpenAtom, versionAtom } from '@/atoms/applicationAtoms'
import { activePageAtom } from '@/atoms/sidebarAtoms'
import { PAGE_DEFINITIONS } from '@/components/sidebar/pageDefinitions.ts'
import { UserBar } from '@/components/sidebar/UserBar'
import { cn } from '@/lib/utils'
import { useAddonStore } from '@/stores/addonStore'

import { SidebarItem } from './SidebarItem'

export const Sidebar = () => {
  const [activeItem, setActiveItem] = useAtom(activePageAtom)
  const version = useAtomValue(versionAtom)
  const updateAvailable = useAtomValue(updateAvailableAtom)
  const [, setUpdateDialogOpen] = useAtom(updateDialogOpenAtom)
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
            icon={Globe}
            isActive={false}
            onClick={() => Browser.OpenURL('https://aa-classic.com')}
          />
        </nav>
      </div>

      {updateAvailable && (
        <button
          type="button"
          onClick={() => setUpdateDialogOpen(true)}
          className="mx-2 mb-1 flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-3 py-1.5 text-xs text-emerald-400 select-none cursor-pointer hover:bg-emerald-500/20 transition-colors"
        >
          <ArrowUpCircle className="w-3.5 h-3.5 shrink-0" />
          <span className="truncate">Update Available</span>
        </button>
      )}

      <div className="w-full">
        <UserBar />
      </div>

      <div className="mx-3 mb-3 mt-0.5">
        <button
          type="button"
          onClick={() =>
            Browser.OpenURL(
              `https://github.com/classic-addon-manager/classic-addon-manager/releases/tag/v${version}`
            )
          }
          aria-label={`Version ${version}, open release notes`}
          className={cn(
            'group flex w-full items-center justify-center gap-2 rounded-lg px-2 py-1.5',
            'text-[11px] leading-none text-muted-foreground/60',
            'outline-none transition-colors duration-200',
            'hover:bg-secondary/40 hover:text-muted-foreground',
            'focus-visible:ring-1 focus-visible:ring-ring'
          )}
        >
          <span className="font-mono tabular-nums tracking-tight text-muted-foreground/80 transition-colors group-hover:text-foreground/80">
            v{version}
          </span>
          <span
            aria-hidden
            className="h-2.5 w-px shrink-0 bg-border/70 transition-colors group-hover:bg-border"
          />
          <span className="tracking-wide">
            made with <span aria-hidden>❤️</span> by Sami
          </span>
        </button>
      </div>
    </>
  )
}
