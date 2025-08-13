import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue, useSetAtom } from 'jotai'
import { Package, PackageSearch } from 'lucide-react'

import { listAnimations } from '@/animations/listAnimations'
import { activePageAtom } from '@/atoms/sidebarAtoms'
import { filteredAddonsAtom } from '@/components/dashboard/atoms.ts'
import { LocalAddon } from '@/components/dashboard/LocalAddon.tsx'
import { Button } from '@/components/ui/button'
import { useAddonStore } from '@/stores/addonStore'

export const AddonList = () => {
  const { installedAddons } = useAddonStore()
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const setActiveItem = useSetAtom(activePageAtom)

  if (installedAddons.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border bg-card/50 p-8 text-center min-h-screen-minus-10">
        <div className="rounded-full bg-primary/10 p-4 mb-4">
          <Package className="size-8 text-primary" />
        </div>
        <h3 className="text-2xl font-semibold tracking-tight mb-2">No Addons Installed</h3>
        <p className="text-muted-foreground text-sm max-w-sm mb-6">
          Get started by browsing our collection of addons and enhance your ArcheAge experience.
        </p>
        <Button
          size="lg"
          onClick={() => {
            setActiveItem('addons')
          }}
          className="transition-all duration-200 hover:shadow-md cursor-pointer"
        >
          Browse Addons
        </Button>
      </div>
    )
  }

  return (
    <AnimatePresence mode="wait">
      {filteredAddons.length === 0 ? (
        <motion.div
          key="no-local-addons"
          className="flex flex-col items-center justify-center rounded-lg border bg-card/50 p-10 text-center min-h-[280px]"
          {...listAnimations.emptyState}
        >
          <PackageSearch className="h-12 w-12 mb-4 text-muted-foreground" />
          <h3 className="text-lg font-semibold mb-1">No addons match your search</h3>
          <p className="text-sm text-muted-foreground">
            Try a different query or clear filters to see all installed addons.
          </p>
        </motion.div>
      ) : (
        <motion.main key="local-addon-list" className="flex-1" {...listAnimations.container}>
          <motion.div className="flex flex-1 flex-col gap-3 py-2" layout>
            <AnimatePresence initial={false}>
              {filteredAddons.map(addon => (
                <motion.div key={addon.name} layout {...listAnimations.item}>
                  <LocalAddon addon={addon} />
                </motion.div>
              ))}
            </AnimatePresence>
          </motion.div>
        </motion.main>
      )}
    </AnimatePresence>
  )
}
