import { useAtom } from 'jotai'
import { AlertTriangleIcon, ArrowUpCircle, CheckIcon } from 'lucide-react'
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
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        Good news! This addon was found in our repository. Would you like Classic Addon Manager to
        handle updates for you?
      </p>
      <Button className="w-full sm:w-auto" onClick={handleMatchAddon}>
        <ArrowUpCircle className="w-4 h-4 mr-2" />
        Yes, manage this addon
      </Button>
    </div>
  )
}
