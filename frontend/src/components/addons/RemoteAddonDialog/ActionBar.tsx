import { useIsMutating } from '@tanstack/react-query'
import { DownloadIcon, LoaderCircle, Trash2Icon } from 'lucide-react'

import { RatingButtons } from '@/components/addons/RatingButtons.tsx'
import { Button } from '@/components/ui/button.tsx'
import type { AddonManifest, Release } from '@/lib/wails'

import { addonActionMutationKey } from './useAddonActions.ts'

interface ActionBarProps {
  manifest: AddonManifest
  release: Release | null
  rating: number
  isRatingDisabled: boolean
  isInstalled: boolean
  isLoadingRelease: boolean
  onRate: (rating: number) => Promise<boolean>
  onInstall: () => void
  onUninstall: () => void
}

const InstallLabel = ({
  release,
  isProcessing,
  isLoadingRelease,
}: Pick<ActionBarProps, 'release' | 'isLoadingRelease'> & { isProcessing: boolean }) => {
  if (isLoadingRelease) {
    return (
      <>
        <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
        Checking...
      </>
    )
  }

  if (!release) {
    return <>Not Available</>
  }

  if (isProcessing) {
    return (
      <>
        <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
        Installing...
      </>
    )
  }

  return (
    <>
      <DownloadIcon className="mr-2 h-4 w-4" />
      Install
    </>
  )
}

export const ActionBar = ({
  manifest,
  release,
  rating,
  isRatingDisabled,
  isInstalled,
  isLoadingRelease,
  onRate,
  onInstall,
  onUninstall,
}: ActionBarProps) => {
  const isProcessing = useIsMutating({ mutationKey: addonActionMutationKey(manifest.name) }) > 0

  return (
    <div className="sticky bottom-0 z-20 flex items-center justify-between gap-4 border-t bg-background/80 p-4 backdrop-blur-md">
      <div className="flex items-center gap-1">
        <RatingButtons rating={rating} onRate={onRate} disabled={isRatingDisabled} />
      </div>

      <Button
        variant={isInstalled ? 'destructive' : 'default'}
        onClick={isInstalled ? onUninstall : onInstall}
        disabled={(!isInstalled && (!release || isLoadingRelease)) || isProcessing}
        className="min-w-[120px]"
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
            <Trash2Icon className="mr-2 h-4 w-4" />
            Uninstall
          </>
        ) : (
          <InstallLabel
            release={release}
            isProcessing={isProcessing}
            isLoadingRelease={isLoadingRelease}
          />
        )}
      </Button>
    </div>
  )
}
