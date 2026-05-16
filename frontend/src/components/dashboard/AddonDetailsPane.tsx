import { Browser } from '@wailsio/runtime'
import { clsx } from 'clsx'
import {
  AlertTriangleIcon,
  BlocksIcon,
  BugIcon,
  CheckIcon,
  Download,
  FolderOpen,
  GitBranchIcon,
  GithubIcon,
  LoaderCircle,
  PackageIcon,
  RefreshCw,
  Tag,
  ThumbsDownIcon,
  ThumbsUpIcon,
  Trash2Icon,
  User,
} from 'lucide-react'
import { Suspense, useCallback, useEffect, useState } from 'react'

import { AddonRepositoryMatch } from '@/components/dashboard/AddonRepositoryMatch'
import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { toast } from '@/components/ui/toast'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { getMyRating, rateAddon } from '@/lib/addon'
import { repoGetManifest } from '@/lib/repo'
import { formatToLocalTime, safeCall } from '@/lib/utils'
import type { Addon, AddonManifest } from '@/lib/wails'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'
import { useUpdateDialogStore } from '@/stores/updateDialogStore'
import { useUserStore } from '@/stores/userStore'

interface AddonDetailsPaneProps {
  addon: Addon
  onOpenVersionSelect: (addon: Addon) => void
}

export const AddonDetailsPane = ({ addon, onOpenVersionSelect }: AddonDetailsPaneProps) => {
  return (
    <Suspense fallback={<DetailsLoading />}>
      <AddonDetailsContent addon={addon} onOpenVersionSelect={onOpenVersionSelect} />
    </Suspense>
  )
}

const DetailsLoading = () => (
  <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
    <LoaderCircle className="size-10 animate-spin opacity-50" strokeWidth={1.5} />
    <p className="mt-3 text-sm">Loading details...</p>
  </div>
)

const AddonDetailsContent = ({ addon, onOpenVersionSelect }: AddonDetailsPaneProps) => {
  const { install, uninstall, latestReleasesMap, isCheckingForUpdates } = useAddonStore()
  const { isAuthenticated } = useUserStore()
  const { open: updateDialogOpen, setOpen: setUpdateDialogOpen } = useUpdateDialogStore()
  const [readme, setReadme] = useState<string>('loading')
  const [rating, setRating] = useState(0)

  const latestRelease = latestReleasesMap.get(addon.name)
  const hasUpdate = addon.isManaged && latestRelease && latestRelease.published_at > addon.updatedAt

  const [hasIcon, setHasIcon] = useState(true)
  const iconUrl =
    'repo' in addon && 'branch' in addon
      ? `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`
      : null

  // Reset on addon switch so a failed icon doesn't hide valid icons for subsequent addons
  useEffect(() => {
    setHasIcon(true)
  }, [addon.name, iconUrl])

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

  useEffect(() => {
    handleGetMyRating().catch(e => console.error('Failed to fetch rating: ', e))
    getReadme().catch(e => console.error('Failed to fetch readme: ', e))
  }, [addon.name, handleGetMyRating, getReadme])

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
  }

  const handleOpenDirectory = async () => {
    const [, err] = await safeCall(LocalAddonService.OpenDirectory(addon.name))
    if (err) {
      toast({
        icon: AlertTriangleIcon,
        title: 'Error',
        description: `Failed to open directory for addon "${addon.alias}".`,
      })
    }
  }

  const RateAddonButtons = () => {
    if (!addon.isManaged) return null

    if (!isAuthenticated()) {
      return (
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              className="flex items-center gap-3 p-2 cursor-not-allowed opacity-50"
              aria-label="Log in to rate addons"
            >
              <ThumbsUpIcon className="w-4 h-4 text-muted-foreground" />
              <ThumbsDownIcon className="w-4 h-4 text-muted-foreground" />
            </span>
          </TooltipTrigger>
          <TooltipContent className="font-bold">
            Sign in using Discord to rate addons
          </TooltipContent>
        </Tooltip>
      )
    }

    return (
      <div className="flex gap-1 items-center">
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-7 w-7 transition-all duration-200 hover:bg-blue-100 dark:hover:bg-blue-900/30',
            rating === 1 && 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500 text-blue-500'
          )}
          onClick={() => handleRateAddon(1)}
          aria-label="Like addon"
        >
          <ThumbsUpIcon
            className={clsx('w-4 h-4', {
              'text-blue-500': rating === 1,
              'text-muted-foreground': rating !== 1,
            })}
          />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className={clsx(
            'h-7 w-7 transition-all duration-200 hover:bg-red-100 dark:hover:bg-red-900/30',
            rating === -1 && 'bg-red-100 dark:bg-red-900/30 border border-red-500 text-red-500'
          )}
          onClick={() => handleRateAddon(-1)}
          aria-label="Dislike addon"
        >
          <ThumbsDownIcon
            className={clsx('w-4 h-4', {
              'text-red-500': rating === -1,
              'text-muted-foreground': rating !== -1,
            })}
          />
        </Button>
      </div>
    )
  }

  const UnmanagedNotice = () => {
    if (addon.isManaged) return null
    return (
      <div className="border rounded-lg p-4 bg-card">
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
                <div className="w-3 h-3 rounded-full border-2 border-t-transparent border-current animate-spin" />
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

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="shrink-0 border-b px-6 py-5">
        <div className="flex items-start gap-4">
          <div className="shrink-0">
            {iconUrl && hasIcon ? (
              <img
                className="h-14 w-14 rounded-xl object-cover border border-border/50 shadow-sm"
                src={iconUrl}
                alt={`${addon.alias} icon`}
                loading="lazy"
                onError={() => setHasIcon(false)}
              />
            ) : (
              <div className="flex items-center justify-center h-14 w-14 rounded-xl bg-muted border border-border/50">
                <BlocksIcon className="h-7 w-7 opacity-40 stroke-[1.5]" />
              </div>
            )}
          </div>

          <div className="flex-1 min-w-0">
            <h2 className="text-xl font-semibold truncate">{addon.alias}</h2>

            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 mt-1 text-sm text-muted-foreground">
              {addon.isManaged && (
                <span className="inline-flex items-center gap-1">
                  <User className="w-3.5 h-3.5" />
                  <span className="text-foreground/90">{addon.author}</span>
                </span>
              )}

              {addon.isManaged && addon.version && (
                <Badge variant="secondary" className="inline-flex items-center gap-1 text-xs">
                  <Tag className="w-3 h-3" />
                  {addon.version}
                </Badge>
              )}

              {!addon.isManaged && (
                <Badge
                  variant="outline"
                  className="text-xs text-orange-600 border-orange-600/20 bg-orange-500/10"
                >
                  Manual
                </Badge>
              )}

              {hasUpdate && (
                <Badge
                  variant="outline"
                  className="text-xs text-amber-600 border-amber-600/20 bg-amber-500/10 cursor-pointer"
                  onClick={() => {
                    if (!updateDialogOpen) setUpdateDialogOpen(true)
                  }}
                >
                  <Download className="w-3 h-3 mr-0.5" />
                  Update ({latestRelease!.tag_name})
                </Badge>
              )}

              {addon.isManaged && !hasUpdate && latestRelease && !isCheckingForUpdates && (
                <Badge
                  variant="outline"
                  className="text-xs text-green-600 border-green-600/20 bg-green-500/10"
                >
                  Up to date
                </Badge>
              )}
            </div>
          </div>

          <RateAddonButtons />
        </div>
      </div>

      <div className="shrink-0 border-b px-6 py-3">
        <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
          <div>
            <span className="text-muted-foreground">Name</span>
            <p className="font-medium truncate">{addon.name}</p>
          </div>

          {addon.isManaged && (
            <>
              <div>
                <span className="text-muted-foreground">Updated</span>
                <p className="font-medium truncate">
                  {addon.updatedAt ? formatToLocalTime(addon.updatedAt, 'short') : '—'}
                </p>
              </div>
              <div>
                <span className="text-muted-foreground">Source</span>
                {addon.repo ? (
                  <a
                    href={`https://github.com/${addon.repo}`}
                    onClick={e => {
                      e.preventDefault()
                      Browser.OpenURL(`https://github.com/${addon.repo}`)
                    }}
                    className="font-medium text-blue-400 hover:text-blue-500 hover:underline truncate block"
                  >
                    {addon.repo}
                  </a>
                ) : (
                  <p className="font-medium">—</p>
                )}
              </div>
            </>
          )}
          {!addon.isManaged && (
            <div>
              <span className="text-muted-foreground">State</span>
              <p className="font-medium">Not managed</p>
            </div>
          )}
        </div>
      </div>

      <div className="shrink-0 px-6 py-3 border-b">
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-8 text-xs gap-1.5"
            onClick={handleOpenDirectory}
          >
            <FolderOpen className="w-3.5 h-3.5" />
            Open Directory
          </Button>

          {addon.isManaged && (
            <>
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs gap-1.5"
                onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}
              >
                <GithubIcon className="w-3.5 h-3.5" />
                View Code
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs gap-1.5"
                onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
              >
                <BugIcon className="w-3.5 h-3.5" />
                Report Issue
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs gap-1.5"
                onClick={() => onOpenVersionSelect(addon)}
              >
                <GitBranchIcon className="w-3.5 h-3.5" />
                Other Version
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs gap-1.5"
                onClick={handleReinstall}
              >
                <RefreshCw className="w-3.5 h-3.5" />
                Reinstall
              </Button>
            </>
          )}

          <Button
            variant="destructive"
            size="sm"
            className="h-8 text-xs gap-1.5"
            onClick={handleUninstall}
          >
            <Trash2Icon className="w-3.5 h-3.5" />
            Uninstall
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1 min-h-0 px-6 py-4">
        {addon.isManaged && readme === 'loading' ? (
          <div className="flex flex-col items-center justify-center h-32 text-center text-muted-foreground">
            <PackageIcon className="w-10 h-10 mb-3 opacity-50" />
            <p className="text-sm font-medium">Loading description...</p>
          </div>
        ) : addon.isManaged ? (
          <RemoteAddonReadme readme={readme} />
        ) : null}

        <UnmanagedNotice />
      </ScrollArea>
    </div>
  )
}
