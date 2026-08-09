import { useEffect, useRef } from 'react'

import { Dialog, DialogContent } from '@/components/ui/dialog'
import type { AddonManifest } from '@/lib/wails'

import { ActionBar } from './RemoteAddonDialog/ActionBar.tsx'
import { ChangelogSection } from './RemoteAddonDialog/ChangelogSection.tsx'
import { DescriptionSection } from './RemoteAddonDialog/DescriptionSection.tsx'
import { Header } from './RemoteAddonDialog/Header.tsx'
import { RequiresSection } from './RemoteAddonDialog/RequiresSection.tsx'
import { ScrollToTop } from './RemoteAddonDialog/ScrollToTop.tsx'
import { StatBar } from './RemoteAddonDialog/StatBar.tsx'
import { SupportSection } from './RemoteAddonDialog/SupportSection.tsx'
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
  const scrollRef = useRef<HTMLDivElement>(null)
  const {
    release,
    readme,
    changelog,
    rating,
    dependencies,
    isInstalled,
    isProcessing,
    isLoadingRelease,
    isLoadingReadme,
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

  useEffect(() => {
    if (!open) return
    checkInstalledStatus().catch(e => {
      console.error('Failed to check installed status:', e)
    })
  }, [open, checkInstalledStatus])

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

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex max-h-[90svh] min-h-[500px] w-full max-w-0 min-w-[650px] flex-col gap-0 overflow-hidden p-0 md:max-w-[70svw] lg:max-w-[850px]"
        onOpenAutoFocus={event => {
          // Focusing the first control would pop its tooltip open on every dialog open.
          event.preventDefault()
          if (event.currentTarget instanceof HTMLElement) {
            event.currentTarget.focus({ preventScroll: true })
          }
        }}
      >
        <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
          <div className="flex min-h-full flex-col">
            <div className="flex-1">
              <Header manifest={manifest} release={release} />
              <Warning text={manifest.warning} />
              <StatBar
                manifest={manifest}
                release={release}
                isLoadingRelease={isLoadingRelease}
              />

              <DescriptionSection readme={readme} isLoading={isLoadingReadme} />
              <ChangelogSection
                release={release}
                changelog={changelog}
                isLoading={isLoadingRelease}
              />
              {dependencies.length > 0 && (
                <RequiresSection
                  dependencies={dependencies}
                  onDependencyClick={handleDependencyClick}
                />
              )}
              {manifest.kofi && <SupportSection manifest={manifest} />}
            </div>

            <ActionBar
              manifest={manifest}
              release={release}
              rating={rating}
              isInstalled={isInstalled}
              isProcessing={isProcessing}
              isLoadingRelease={isLoadingRelease}
              onRate={rateAddon}
              onInstall={handleInstall}
              onUninstall={handleUninstall}
            />
          </div>
        </div>

        <ScrollToTop scrollRef={scrollRef} />
      </DialogContent>
    </Dialog>
  )
}
