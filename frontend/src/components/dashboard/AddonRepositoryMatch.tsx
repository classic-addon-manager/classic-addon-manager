import { useAtom } from 'jotai'
import { AlertTriangleIcon, ArrowUpCircle, CheckIcon, CloudOffIcon } from 'lucide-react'
import usePromise from 'react-promise-suspense'

import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast'
import { repoGetManifest } from '@/lib/repo'
import { safeCall } from '@/lib/utils.ts'
import type { AddonManifest } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

import { isAddonDialogOpenAtom, selectedAddonAtom } from './atoms'

const fetchData = async (name: string) => {
  const [manifest, err] = await safeCall(repoGetManifest(name))
  if (err) {
    return null
  }

  return manifest
}

export const AddonRepositoryMatch = ({ name }: { name: string }) => {
  const data = usePromise(fetchData, [name])
  const { install, performBulkUpdateCheck } = useAddonStore()
  const [, setDialogOpen] = useAtom(isAddonDialogOpenAtom)
  const [, setSelectedAddon] = useAtom(selectedAddonAtom)

  if (!data) {
    return null
  }

  const handleMatchAddon = async () => {
    const [manifest, manifestErr] = await safeCall<AddonManifest>(repoGetManifest(name))
    if (manifestErr || !manifest) {
      console.error('Failed to fetch addon manifest:', manifestErr)
      toast({
        icon: AlertTriangleIcon,
        title: 'Error',
        description: `Failed to fetch addon manifest for "${name}".`,
      })
      return
    }

    const [, installErr] = await safeCall<boolean>(install(manifest, 'latest'))
    if (installErr) {
      const errorString = String(installErr)
      if (errorString.includes('no release found')) {
        toast({
          icon: AlertTriangleIcon,
          title: 'Error',
          description: `No releases found for addon "${manifest.name}".`,
        })
      } else {
        console.error('Failed to install addon during match:', installErr)
        toast({
          icon: AlertTriangleIcon,
          title: 'Error',
          description: `Failed to match addon "${manifest.name}": ${errorString.substring(0, 100)}`,
        })
      }
      return
    }

    await performBulkUpdateCheck()

    toast({
      icon: CheckIcon,
      title: 'Addon Matched',
      description: `"${manifest.alias}" is now managed by Classic Addon Manager.`,
    })

    setDialogOpen(false)
    setSelectedAddon(null)
  }

  return (
    <div className="border rounded-lg p-4 bg-card flex-1 flex flex-col">
      <div className="flex flex-col items-center justify-center text-center py-8 flex-1">
        <div className="flex items-center justify-center w-12 h-12 rounded-full bg-orange-500/10 mb-4">
          <CloudOffIcon className="w-6 h-6 text-orange-500" />
        </div>
        <p className="text-sm font-medium text-foreground mb-1">Manage This Addon?</p>
        <p className="text-sm text-muted-foreground mb-5 max-w-xs">
          You installed this one yourself.
          <br />
          We can take over updates if you'd like.
        </p>
        <div className="space-y-3">
          <p className="text-sm text-muted-foreground">
            It's in our repository, we can keep it up to date for you.
          </p>
          <Button className="w-full sm:w-auto" onClick={handleMatchAddon}>
            <ArrowUpCircle className="w-4 h-4 mr-2" />
            Yes, keep it updated
          </Button>
        </div>
      </div>
    </div>
  )
}
