import clsx from 'clsx'
import { DownloadIcon, ThumbsDownIcon, ThumbsUpIcon, Trash2Icon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Button } from '@/components/ui/button.tsx'
import { Dialog, DialogContent, DialogFooter } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs.tsx'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip.tsx'
import type { AddonManifest } from '@/lib/wails'
import { useUserStore } from '@/stores/userStore.ts'

import { ChangelogTab } from './RemoteAddonDialog/ChangelogTab.tsx'
import { DependenciesTab } from './RemoteAddonDialog/DependenciesTab.tsx'
import { DescriptionTab } from './RemoteAddonDialog/DescriptionTab.tsx'
import { Header } from './RemoteAddonDialog/Header.tsx'
import { SupportTab } from './RemoteAddonDialog/SupportTab.tsx'
import { useAddonActions } from './RemoteAddonDialog/useAddonActions.ts'
import { Warning } from './RemoteAddonDialog/Warning.tsx'

interface RemoteAddonDialogProps {
  manifest: AddonManifest
  open: boolean
  onOpenChange: (open: boolean) => void
  onViewDependency: (manifest: AddonManifest) => void
  onAddonInstalled?: () => void
  onAddonUninstalled?: () => void
}

export const RemoteAddonDialog = ({
  manifest,
  open,
  onOpenChange,
  onViewDependency,
  onAddonInstalled,
  onAddonUninstalled,
}: RemoteAddonDialogProps) => {
  const [currentTab, setCurrentTab] = useState<string>('description')
  const {
    release,
    readme,
    changelog,
    rating,
    dependencies,
    isInstalled,
    isProcessing,
    checkInstalledStatus,
    handleInstall,
    handleUninstall,
    getRelease,
    getReadme,
    getMyRating,
    getDependencies,
    handleDependencyClick,
    rateAddon,
  } = useAddonActions({
    manifest,
    onViewDependency,
    onOpenChange,
    onAddonInstalled,
    onAddonUninstalled,
  })

  const { isAuthenticated } = useUserStore()

  useEffect(() => {
    if (!open) return
    checkInstalledStatus().catch(e => {
      console.error('Failed to check installed status:', e)
    })
  }, [open, checkInstalledStatus])

  useEffect(() => {
    // Reset to description tab when manifest changes
    setCurrentTab('description')
  }, [manifest])

  useEffect(() => {
    if (!open) {
      return
    }

    getMyRating().catch(e => {
      console.error('Failed to fetch rating: ', e)
    })
    getReadme().catch(e => {
      console.error('Failed to fetch readme: ', e)
    })
    getRelease().catch(e => {
      console.error('Failed to fetch release: ', e)
    })
    getDependencies().catch(e => {
      console.error('Failed to fetch dependencies: ', e)
    })
  }, [open, getMyRating, getReadme, getRelease, getDependencies])

  const tabs = () => {
    const baseTabs = [
      { value: 'description', label: 'Description' },
      { value: 'changelog', label: 'Changelog' },
    ]
    if (manifest.dependencies && manifest.dependencies.length > 0) {
      baseTabs.push({ value: 'dependencies', label: `Dependencies (${dependencies.length})` })
    }
    if (manifest.kofi) {
      baseTabs.push({ value: 'kofi', label: 'Support Author' })
    }
    return baseTabs
  }

  const RatingButtons = () => {
    if (!isAuthenticated()) {
      return (
        <TooltipProvider>
          <Tooltip delayDuration={100}>
            <TooltipTrigger className="cursor-not-allowed opacity-50">
              <span className="flex items-center gap-1 p-2">
                <ThumbsUpIcon className="w-5 h-5 text-muted-foreground" />
                <ThumbsDownIcon className="w-5 h-5 text-muted-foreground" />
              </span>
            </TooltipTrigger>
            <TooltipContent>
              <p>Log in to rate addons</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )
    }

    return (
      <>
        <TooltipProvider>
          <Tooltip delayDuration={100}>
            <TooltipTrigger>
              <Button
                variant="ghost"
                size="icon"
                className={clsx(
                  'h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-blue-100 dark:hover:bg-blue-900/30',
                  rating === 1 && 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500'
                )}
                onClick={() => rateAddon(1)}
                aria-label="Like addon"
              >
                <ThumbsUpIcon
                  className={clsx(
                    'w-5 h-5',
                    rating === 1
                      ? 'text-blue-500'
                      : 'text-muted-foreground group-hover:text-blue-500'
                  )}
                />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{rating === 1 ? 'Unlike' : 'Like'}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
          <Tooltip delayDuration={100}>
            <TooltipTrigger>
              <Button
                variant="ghost"
                size="icon"
                className={clsx(
                  'h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-red-100 dark:hover:bg-red-900/30',
                  rating === -1 && 'bg-red-100 dark:bg-red-900/30 border border-red-500'
                )}
                onClick={() => rateAddon(-1)}
                aria-label="Dislike addon"
              >
                <ThumbsDownIcon
                  className={clsx(
                    'w-5 h-5',
                    rating === -1
                      ? 'text-red-500'
                      : 'text-muted-foreground group-hover:text-red-500'
                  )}
                />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>{rating === -1 ? 'Remove Dislike' : 'Dislike'}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </>
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-0 min-w-[650px] md:max-w-[70svw] min-h-[500px] max-h-[90svh] lg:max-w-[850px] flex flex-col p-0">
        <Header manifest={manifest} release={release} />
        <Warning text={manifest.warning} />

        <div className="flex-1 flex flex-col min-h-0 overflow-y-hidden px-6 pb-0">
          <Tabs
            className="w-full flex flex-col min-h-0"
            value={currentTab}
            onValueChange={setCurrentTab}
          >
            <TabsList className="inline-flex items-center bg-transparent justify-start gap-x-4 w-full mb-4">
              {tabs().map(tab => (
                <TabsTrigger
                  key={tab.value}
                  value={tab.value}
                  className="inline-flex items-center justify-center whitespace-nowrap px-3 py-2 text-sm font-medium transition-all focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 border-transparent data-[state=active]:border-primary data-[state=active]:text-primary text-muted-foreground hover:text-primary"
                >
                  <span className="hover:text-foreground transition-colors">{tab.label}</span>
                </TabsTrigger>
              ))}
            </TabsList>

            <div className="flex-1 min-h-0 overflow-y-auto pb-2 pr-2 -mr-2">
              <TabsContent
                value="description"
                className="max-w-none"
                style={{ transform: 'translate(0)' }}
              >
                <DescriptionTab readme={readme} />
              </TabsContent>

              <TabsContent value="changelog" className="text-sm">
                <ChangelogTab release={release} changelog={changelog} />
              </TabsContent>

              {dependencies.length > 0 && (
                <TabsContent value="dependencies">
                  <DependenciesTab
                    dependencies={dependencies}
                    onDependencyClick={handleDependencyClick}
                  />
                </TabsContent>
              )}
              {manifest.kofi && (
                <TabsContent value="kofi">
                  <SupportTab manifest={manifest} />
                </TabsContent>
              )}
            </div>
          </Tabs>
        </div>

        <DialogFooter className="p-4 border-t shrink-0">
          <div className="flex justify-between items-center w-full gap-4">
            <div className="flex gap-1 items-center">
              <RatingButtons />
            </div>
            <Button
              variant={isInstalled ? 'destructive' : 'default'}
              onClick={isInstalled ? handleUninstall : handleInstall}
              disabled={(!isInstalled && !release) || isProcessing}
              className="min-w-[100px]"
              aria-label={
                isInstalled
                  ? `Uninstall ${manifest.alias}`
                  : !release
                    ? 'Addon not available for installation'
                    : `Install ${manifest.alias}`
              }
            >
              {isInstalled ? (
                <>
                  <Trash2Icon className="w-4 h-4 mr-2" />
                  Uninstall
                </>
              ) : !release ? (
                <>Not Available</>
              ) : (
                <>
                  <DownloadIcon className="w-4 h-4 mr-2" />
                  Install
                </>
              )}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
