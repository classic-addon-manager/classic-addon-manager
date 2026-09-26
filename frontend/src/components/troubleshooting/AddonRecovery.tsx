import { AlertTriangleIcon, CheckIcon } from 'lucide-react'
import { useState } from 'react'

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast'
import { safeCall } from '@/lib/utils'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const AddonRecovery = () => {
  const { installedAddons, uninstall, refreshAfterLocalChange } = useAddonStore()
  const [resetting, setResetting] = useState(false)
  const [uninstalling, setUninstalling] = useState(false)
  const [confirm, setConfirm] = useState<'reset' | 'uninstall' | null>(null)

  const resetAddonSettings = async (): Promise<boolean> => {
    const [, err] = await safeCall(LocalAddonService.ResetSettings())
    if (err) {
      console.error('Failed to reset addon settings:', err)
      toast({
        title: 'Error',
        description: 'Failed to reset addon settings',
        icon: AlertTriangleIcon,
      })
      return false
    }
    toast({
      title: 'Addon settings reset',
      description: 'Addon settings reset. Restart the game.',
      icon: CheckIcon,
    })
    return true
  }

  const onResetAddonSettings = async () => {
    if (resetting) return
    setResetting(true)
    try {
      const ok = await resetAddonSettings()
      if (ok) setConfirm(null)
    } finally {
      setResetting(false)
    }
  }

  const onUninstallAllAddons = async () => {
    if (uninstalling) return
    setUninstalling(true)
    try {
      let i = 0
      for (const a of installedAddons) {
        let uninstalled = false
        try {
          uninstalled = await uninstall(a, { skipRefresh: true })
        } catch {
          uninstalled = false
        }
        if (!uninstalled) {
          toast({
            title: 'Error',
            description: `Failed to uninstall addon: ${a.alias}`,
            icon: AlertTriangleIcon,
          })
          return
        }
        i++
      }
      toast({
        title: 'Success',
        description: `${i} ${i === 1 ? 'addon' : 'addons'} uninstalled. Restart the game.`,
        icon: CheckIcon,
      })
      setConfirm(null)

      await resetAddonSettings()
    } finally {
      await refreshAfterLocalChange()
      setUninstalling(false)
    }
  }

  return (
    <>
      <div className="divide-y divide-border/60">
        <div className="flex items-center justify-between gap-6 px-4 py-3.5">
          <div className="min-w-0 space-y-1">
            <div className="text-sm font-medium leading-none">Reset addon settings</div>
            <p className="text-xs text-muted-foreground">Clears addon_settings. Try this first.</p>
          </div>
          <Button
            variant="outline"
            size="sm"
            className="shrink-0"
            onClick={() => setConfirm('reset')}
            disabled={resetting}
          >
            Reset
          </Button>
        </div>

        <div className="flex items-center justify-between gap-6 px-4 py-3.5">
          <div className="min-w-0 space-y-1">
            <div className="text-sm font-medium leading-none">Uninstall all addons</div>
            <p className="text-xs text-muted-foreground">
              {installedAddons.length === 0
                ? 'No addons installed'
                : 'Removes every installed addon, then resets settings. Last resort.'}
            </p>
          </div>
          <Button
            variant="destructive"
            size="sm"
            className="shrink-0"
            onClick={() => setConfirm('uninstall')}
            disabled={installedAddons.length === 0}
          >
            Uninstall all
          </Button>
        </div>
      </div>

      <AlertDialog
        open={confirm === 'reset'}
        onOpenChange={next => {
          if (!resetting) setConfirm(next ? 'reset' : null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Reset addon settings?</AlertDialogTitle>
            <AlertDialogDescription>
              This clears the addon_settings file. Your addons stay installed, but their saved
              settings are lost.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={resetting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={resetting}
              onClick={event => {
                event.preventDefault()
                void onResetAddonSettings()
              }}
            >
              {resetting ? 'Resetting...' : 'Reset settings'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog
        open={confirm === 'uninstall'}
        onOpenChange={next => {
          if (!uninstalling) setConfirm(next ? 'uninstall' : null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Uninstall all addons?</AlertDialogTitle>
            <AlertDialogDescription>
              This removes all {installedAddons.length} installed{' '}
              {installedAddons.length === 1 ? 'addon' : 'addons'} and resets addon settings. You can
              reinstall them later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={uninstalling}>Keep addons</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={uninstalling}
              onClick={event => {
                event.preventDefault()
                void onUninstallAllAddons()
              }}
            >
              {uninstalling ? 'Uninstalling...' : 'Uninstall all'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
