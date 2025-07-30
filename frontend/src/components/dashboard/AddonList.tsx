import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue } from 'jotai'
import { Package } from 'lucide-react'

import { filteredAddonsAtom } from '@/components/dashboard/atoms.ts'
import { LocalAddon } from '@/components/dashboard/LocalAddon.tsx'
import { Button } from '@/components/ui/button'
import { useAddonStore } from '@/stores/addonStore'

export const AddonList = () => {
  const { installedAddons } = useAddonStore()
  const filteredAddons = useAtomValue(filteredAddonsAtom)

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
            // TODO: Navigate to browse addons
          }}
          className="transition-all duration-200 hover:shadow-md"
        >
          Browse Addons
        </Button>
      </div>
    )
  }

  return (
    <div className="rounded-lg border bg-card/10 shadow-sm flex flex-col">
      <div className="grid grid-cols-4 w-full p-2 border-b bg-muted/40">
        <div className="font-medium text-sm text-muted-foreground">Name</div>
        <div className="text-center font-medium text-sm text-muted-foreground">Author</div>
        <div className="text-center font-medium text-sm text-muted-foreground">Version</div>
        <div className="text-center font-medium text-sm text-muted-foreground">Status</div>
      </div>
      <div className="overflow-y-auto">
        <div className="divide-y overflow-clip">
          <AnimatePresence initial={false} mode="sync">
            {filteredAddons.map(addon => (
              <motion.div
                key={addon.name}
                initial={{ opacity: 0 }}
                animate={{
                  opacity: 1,
                  transition: { duration: 0.3 },
                }}
                exit={{
                  opacity: 0,
                  transition: { duration: 0.2 },
                }}
                className="transition-all duration-300 hover:bg-muted/20"
              >
                <LocalAddon addon={addon} />
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      </div>
    </div>
  )
}
