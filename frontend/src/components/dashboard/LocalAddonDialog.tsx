import { Browser } from '@wailsio/runtime'
import { clsx } from 'clsx'
import { useAtom } from 'jotai'
import {
  AlertTriangleIcon,
  BugIcon,
  CheckIcon,
  GithubIcon,
  PackageIcon,
  RefreshCw,
  Tag,
  ThumbsDownIcon,
  ThumbsUpIcon,
  Trash2,
  User,
} from 'lucide-react'
import { Suspense, useCallback, useEffect, useState } from 'react'

import { AddonRepositoryMatch } from '@/components/dashboard/AddonRepositoryMatch'
import { isAddonDialogOpenAtom, selectedAddonAtom } from '@/components/dashboard/atoms'
import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { getMyRating, rateAddon } from '@/lib/addon'
import { repoGetManifest } from '@/lib/repo'
import { safeCall } from '@/lib/utils'
import type { Addon, AddonManifest } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'
import { useUserStore } from '@/stores/userStore'

interface LocalAddonDialogProps {
  addon: Addon
  onOpenChange: (open: boolean) => void
  open: boolean
}

export const LocalAddonDialog = ({ addon, onOpenChange, open }: LocalAddonDialogProps) => {
  const [readme, setReadme] = useState<string>('loading')
  const [initiallyManaged, setInitiallyManaged] = useState(false)
  const [hasBeenOpened, setHasBeenOpened] = useState(false)
  const [rating, setRating] = useState(0)
  const [, setSelectedAddon] = useAtom(selectedAddonAtom)
  const [, setIsDialogOpen] = useAtom(isAddonDialogOpenAtom)
  const { isAuthenticated } = useUserStore()
  const { install, uninstall } = useAddonStore()

  // Set initial managed state when dialog opens
  useEffect(() => {
    if (open && !hasBeenOpened) {
      // Mark that we've opened the dialog and captured the initial state
      setHasBeenOpened(true)
      setInitiallyManaged(addon.isManaged)
    } else if (!open) {
      // Reset when dialog closes
      setHasBeenOpened(false)
    }
  }, [open, addon.isManaged, hasBeenOpened])

  // Close dialog when addon becomes managed (if it wasn't initially managed)
  useEffect(() => {
    // Only close if:
    // 1. Dialog is open
    // 2. We've already captured the initial state (hasBeenOpened)
    // 3. Addon is now managed
    // 4. It wasn't managed initially
    if (open && hasBeenOpened && addon.isManaged && !initiallyManaged) {
      onOpenChange(false)
    }
  }, [addon.isManaged, onOpenChange, open, initiallyManaged, hasBeenOpened])

  const getReadme = useCallback(async () => {
    const fallbackReadme = addon.description || 'No description provided'

    if (!addon.repo) {
      setReadme(fallbackReadme)
      return
    }

    const branch = addon.branch || 'main'

    const response = await fetch(
      `https://raw.githubusercontent.com/${addon.repo}/${branch}/README.md`
    )
    if (!response.ok) {
      setReadme(fallbackReadme)
      return
    }

    const text = await response.text()
    setReadme(text)
  }, [addon.repo, addon.branch, addon.description])

  const handleGetMyRating = useCallback(async () => {
    await getMyRating(addon.name, isAuthenticated(), setRating)
  }, [addon.name, isAuthenticated])

  // Fetch readme when dialog opens
  useEffect(() => {
    if (!open) {
      return
    }
    handleGetMyRating().catch(e => {
      console.error('Failed to fetch rating: ', e)
    })
    getReadme().catch(e => {
      console.error('Failed to fetch readme: ', e)
    })
  }, [open, handleGetMyRating, getReadme])

  const handleRateAddon = async (newRating: number) => {
    await rateAddon(addon.name, addon.alias, newRating, rating, setRating)
  }

  const handleReinstall = async () => {
    const [manifest, manifestError] = await safeCall<AddonManifest>(repoGetManifest(addon.name))

    if (manifestError || !manifest) {
      if (manifestError?.message.includes('no release found')) {
        toast({
          title: 'Error',
          description: 'No release found for this addon',
          icon: AlertTriangleIcon,
        })
      } else {
        toast({
          title: 'Error',
          description: 'Failed to fetch manifest during reinstall, try again later',
        })
      }
      return
    }

    const didInstall: boolean = await install(manifest, 'latest')
    if (!didInstall) return

    toast({
      title: 'Addon reinstalled',
      description: `${addon.alias} was reinstalled`,
      icon: CheckIcon,
    })

    setSelectedAddon(null)
    setIsDialogOpen(false)
  }

  const handleUninstall = async () => {
    const didUninstall = await uninstall(addon)
    if (!didUninstall) {
      toast({
        title: 'Error',
        description: 'Failed to uninstall addon, check log file for more information',
        icon: AlertTriangleIcon,
      })
      return
    }
    toast({
      title: 'Addon uninstalled',
      description: `${addon.alias} was uninstalled`,
      icon: CheckIcon,
    })

    setIsDialogOpen(false)
    setSelectedAddon(null)
  }

  const UnmanagedAddonNotice = () => {
    return (
      <div className="border rounded-lg p-4 bg-card mt-6">
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-muted-foreground">
            <PackageIcon className="w-4 h-4" />
            <p className="text-sm font-medium">
              This addon is not managed by Classic Addon Manager
            </p>
          </div>

          <Suspense
            fallback={
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <div className="w-3 h-3 rounded-full border-2 border-t-transparent border-current animate-spin"></div>
                <span>Checking repository...</span>
              </div>
            }
          >
            <AddonRepositoryMatch name={addon.name} />
          </Suspense>
        </div>
      </div>
    )
  }

  const RateAddonButtons = () => {
    if (!addon.isManaged) {
      return <div></div>
    }

    if (!isAuthenticated) {
      return (
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              className="flex items-center gap-3 p-2 cursor-not-allowed opacity-50"
              aria-label="Log in to rate addons"
            >
              <ThumbsUpIcon className="w-5 h-5 text-muted-foreground" />
              <ThumbsDownIcon className="w-5 h-5 text-muted-foreground" />
            </span>
          </TooltipTrigger>
          <TooltipContent className="font-bold">Log in to rate addons</TooltipContent>
        </Tooltip>
      )
    }

    return (
      <div className="flex gap-2 items-center">
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-8 w-9 transition-all duration-200 hover:bg-blue-100 dark:hover:bg-blue-900/30',
            rating === 1 && 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500 text-blue-500'
          )}
          onClick={() => handleRateAddon(1)}
          aria-label="Like addon"
        >
          <ThumbsUpIcon
            className={clsx('w-5 h-5', {
              'text-blue-500': rating === 1,
              'text-muted-foreground': rating !== 1,
            })}
          />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-8 w-9 transition-all duration-200 hover:bg-red-100 dark:hover:bg-red-900/30',
            rating === -1 && 'bg-red-100 dark:bg-red-900/30 border border-red-500 text-red-500'
          )}
          onClick={() => handleRateAddon(-1)}
          aria-label="Dislike addon"
        >
          <ThumbsDownIcon
            className={clsx('w-5 h-5', {
              'text-red-500': rating === -1,
              'text-muted-foreground': rating !== -1,
            })}
          />
        </Button>
      </div>
    )
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-w-0 min-w-[650px] md:max-w-[70svw] min-h-[500px] max-h-[90svh] lg:max-w-[850px] flex flex-col p-0">
        <DialogHeader className="p-6 pb-4 shrink-0 border-b">
          <DialogTitle className="text-2xl font-semibold mb-1">{addon.alias}</DialogTitle>

          {/* Display author, version, and description information for managed addons */}
          {addon.isManaged && (
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-1 text-muted-foreground">
              <span className="inline-flex items-center gap-1">
                <User className="w-3.5 h-3.5" />
                <span className="font-normal text-foreground/90">{addon.author}</span>
              </span>

              {addon.version ? (
                <Badge variant="secondary" className="inline-flex items-center gap-1">
                  <Tag className="w-3 h-3" />
                  {addon.version}
                </Badge>
              ) : (
                <Badge variant="outline">No Version</Badge>
              )}

              {addon.description && (
                <DialogDescription className="text-sm text-muted-foreground">
                  {addon.description}
                </DialogDescription>
              )}
            </div>
          )}

          {addon.isManaged && (
            <div className="flex flex-wrap gap-2 text-sm">
              <Button
                variant="outline"
                size="sm"
                className="flex items-center gap-1"
                onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}
              >
                <GithubIcon className="w-4 h-4" />
                View Code
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="flex items-center gap-1"
                onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
              >
                <BugIcon className="w-4 h-4" />
                Report Issue
              </Button>
            </div>
          )}
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 pb-6 pt-2">
          {addon.isManaged && readme === 'loading' ? (
            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
              <PackageIcon className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">Loading description...</p>
            </div>
          ) : (
            addon.isManaged && <RemoteAddonReadme readme={readme} />
          )}

          {/* Show notice if addon is not managed and is available in repository */}
          {!addon.isManaged && UnmanagedAddonNotice()}
        </div>

        <DialogFooter className="p-4 border-t">
          <div className="flex justify-between items-center w-full gap-4">
            {RateAddonButtons()}

            <div className="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
              {addon.isManaged && (
                <Button
                  type="button"
                  variant="outline"
                  className="flex items-center"
                  onClick={handleReinstall}
                >
                  <RefreshCw className="w-4 h-4" />
                  Reinstall
                </Button>
              )}
              <Button
                type="button"
                variant="destructive"
                className="flex items-center"
                onClick={handleUninstall}
              >
                <Trash2 className="w-4 h-4" />
                Uninstall
              </Button>
            </div>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
