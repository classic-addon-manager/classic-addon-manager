import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue, useSetAtom } from 'jotai'
import { AlertTriangle, PackageSearch } from 'lucide-react'
import type { ReactNode } from 'react'

import { listAnimations } from '@/animations/listAnimations'
import { AddonSkeleton } from '@/components/addons/AddonSkeleton'
import { RemoteAddon } from '@/components/addons/RemoteAddon'
import { ListHeader } from '@/components/addons/RemoteAddon/List'
import { listGroupClass } from '@/components/addons/RemoteAddon/listLayout'
import { Button } from '@/components/ui/button'
import { isNewAddon, NEW_ADDON_DAYS } from '@/lib/utils'
import { usePreferencesStore } from '@/stores/preferencesStore'

import {
  addonsAtom,
  addonsErrorAtom,
  filteredAddonsAtom,
  isAddonsReadyAtom,
  loadAddonsAtom,
  searchQueryAtom,
} from './atoms'
import type { AddonListItem } from './types'

const listContainerClass = 'flex flex-1 flex-col py-4'
const gridContainerClass = 'flex flex-1 flex-col py-4'
// Cards need about 13.75rem for the stats footer to fit next to "Installed".
const gridClass = 'grid grid-cols-[repeat(auto-fill,minmax(13.75rem,1fr))] gap-3'

const SectionLabel = ({
  highlight = false,
  className = 'mx-3 mb-1.5 mt-3',
  children,
}: {
  highlight?: boolean
  className?: string
  children: ReactNode
}) => (
  <div
    className={`${className} flex items-center gap-2.5 text-[11px] uppercase tracking-wider text-muted-foreground after:h-px after:flex-1 after:bg-border/60`}
  >
    {highlight && (
      <span className="h-1.5 w-1.5 rounded-full bg-primary ring-[3px] ring-primary/20" />
    )}
    {children}
  </div>
)

export const AddonList = () => {
  const isReady = useAtomValue(isAddonsReadyAtom)
  const addonsError = useAtomValue(addonsErrorAtom)
  const loadAddons = useSetAtom(loadAddonsAtom)
  const addons = useAtomValue(addonsAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const searchQuery = useAtomValue(searchQueryAtom)
  const viewMode = usePreferencesStore(s => s.addonViewMode)

  if (!isReady) {
    const skeletons = Array(7)
      .fill(null)
      .map((_, i) => <AddonSkeleton key={i} variant={viewMode} />)

    return viewMode === 'grid' ? (
      <div className={`${gridContainerClass} relative`}>
        <div className={gridClass}>{skeletons}</div>
      </div>
    ) : (
      <div className={`${listContainerClass} relative`}>
        <ListHeader />
        <div className={listGroupClass}>{skeletons}</div>
      </div>
    )
  }

  // The catalog is sorted with new addons first, so each section keeps that order.
  const newAddons = filteredAddons.filter(addon => isNewAddon(addon.manifest.added_at))
  const otherAddons = filteredAddons.filter(addon => !isNewAddon(addon.manifest.added_at))

  const renderItems = (items: AddonListItem[]) => (
    <AnimatePresence initial={false}>
      {items.map(addon => (
        <motion.div key={addon.manifest.name} className="min-w-0" layout {...listAnimations.item}>
          <RemoteAddon manifest={addon.manifest} installed={addon.isInstalled} variant={viewMode} />
        </motion.div>
      ))}
    </AnimatePresence>
  )

  return (
    <>
      {addonsError && addons.length > 0 && (
        <div className="mt-4 flex items-center gap-2 rounded-lg border bg-card px-3 py-2 text-sm text-muted-foreground">
          <AlertTriangle className="h-4 w-4 shrink-0 text-orange-500" />
          <span className="min-w-0 flex-1">
            Couldn't refresh addons: {addonsError.substring(0, 120)}. Showing the last loaded list.
          </span>
          <Button variant="ghost" size="sm" onClick={() => loadAddons(true)}>
            Retry
          </Button>
        </div>
      )}
      <AnimatePresence mode="wait">
        {addonsError && addons.length === 0 ? (
          <motion.div
            key="addons-error"
            className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground"
            {...listAnimations.emptyState}
          >
            <AlertTriangle className="h-12 w-12 mb-4" />
            <h3 className="text-lg font-semibold mb-2">Couldn't load addons</h3>
            <p className="max-w-sm">{addonsError.substring(0, 200)}</p>
            <Button variant="outline" size="sm" className="mt-4" onClick={() => loadAddons(true)}>
              Retry
            </Button>
          </motion.div>
        ) : filteredAddons.length === 0 ? (
          <motion.div
            key="no-addons"
            className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground"
            {...listAnimations.emptyState}
          >
            <PackageSearch className="h-12 w-12 mb-4" />
            <h3 className="text-lg font-semibold mb-2">No addons found</h3>
            <p className="max-w-sm">
              {searchQuery
                ? 'No addons match your search criteria. Try adjusting your search or filters.'
                : 'No addons are currently available. Check back later or try refreshing.'}
            </p>
          </motion.div>
        ) : (
          <motion.main
            key={`addon-list-${viewMode}`}
            className="flex-1"
            {...listAnimations.container}
          >
            {viewMode === 'grid' ? (
              <div className={gridContainerClass}>
                {newAddons.length > 0 && (
                  <>
                    <SectionLabel highlight className="mb-2.5">
                      New in the last {NEW_ADDON_DAYS} days
                    </SectionLabel>
                    <motion.div className={gridClass} layout>
                      {renderItems(newAddons)}
                    </motion.div>
                  </>
                )}
                {otherAddons.length > 0 && (
                  <>
                    {newAddons.length > 0 && (
                      <SectionLabel className="mb-2.5 mt-5">All other addons</SectionLabel>
                    )}
                    <motion.div className={gridClass} layout>
                      {renderItems(otherAddons)}
                    </motion.div>
                  </>
                )}
              </div>
            ) : (
              <div className={listContainerClass}>
                <ListHeader />
                {newAddons.length > 0 && (
                  <>
                    <SectionLabel highlight>New in the last {NEW_ADDON_DAYS} days</SectionLabel>
                    <motion.div className={listGroupClass} layout>
                      {renderItems(newAddons)}
                    </motion.div>
                  </>
                )}
                {otherAddons.length > 0 && (
                  <>
                    {newAddons.length > 0 && <SectionLabel>All other addons</SectionLabel>}
                    <motion.div className={listGroupClass} layout>
                      {renderItems(otherAddons)}
                    </motion.div>
                  </>
                )}
              </div>
            )}
          </motion.main>
        )}
      </AnimatePresence>
    </>
  )
}
