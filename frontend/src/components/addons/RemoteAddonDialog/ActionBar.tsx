import { useIsMutating } from '@tanstack/react-query'
import { DownloadIcon, LoaderCircle, ThumbsDownIcon, ThumbsUpIcon, Trash2Icon } from 'lucide-react'

import { Button } from '@/components/ui/button.tsx'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip.tsx'
import { cn } from '@/lib/utils.ts'
import type { AddonManifest, Release } from '@/lib/wails'
import { useUserStore } from '@/stores/userStore.ts'

import { addonActionMutationKey } from './useAddonActions.ts'

interface ActionBarProps {
  manifest: AddonManifest
  release: Release | null
  rating: number
  isRatingDisabled: boolean
  isInstalled: boolean
  isLoadingRelease: boolean
  onRate: (rating: number) => void
  onInstall: () => void
  onUninstall: () => void
}

const RatingButtons = ({
  rating,
  onRate,
  disabled,
}: {
  rating: number
  onRate: (rating: number) => void
  disabled: boolean
}) => {
  const { isAuthenticated } = useUserStore()

  if (!isAuthenticated()) {
    return (
      <TooltipProvider>
        <Tooltip delayDuration={100}>
          <TooltipTrigger className="cursor-not-allowed opacity-50">
            <span className="flex items-center gap-1 p-2">
              <ThumbsUpIcon className="h-5 w-5 text-muted-foreground" />
              <ThumbsDownIcon className="h-5 w-5 text-muted-foreground" />
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
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className={cn(
                'h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-primary/10',
                rating === 1 && 'bg-primary/10 ring-1 ring-inset ring-primary/40'
              )}
              onClick={() => onRate(1)}
              aria-label="Like addon"
              disabled={disabled}
            >
              <ThumbsUpIcon
                className={cn('h-5 w-5', rating === 1 ? 'text-primary' : 'text-muted-foreground')}
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
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className={cn(
                'h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-destructive/10',
                rating === -1 && 'bg-destructive/10 ring-1 ring-inset ring-destructive/40'
              )}
              onClick={() => onRate(-1)}
              aria-label="Dislike addon"
              disabled={disabled}
            >
              <ThumbsDownIcon
                className={cn(
                  'h-5 w-5',
                  rating === -1 ? 'text-destructive' : 'text-muted-foreground'
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
