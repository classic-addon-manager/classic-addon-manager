import { Browser } from '@wailsio/runtime'
import { BugIcon, GithubIcon } from 'lucide-react'
import { useState } from 'react'

import { Icon } from '@/components/addons/RemoteAddon/Icon.tsx'
import { Button } from '@/components/ui/button.tsx'
import { DialogTitle } from '@/components/ui/dialog'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip.tsx'
import { cn } from '@/lib/utils.ts'
import type { AddonManifest, Release } from '@/lib/wails'

interface HeaderProps {
  manifest: AddonManifest
  release: Release | null
}

export const Header = ({ manifest, release }: HeaderProps) => {
  const bannerUrl = `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/banner.png`
  const iconUrl = `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/icon.png`

  // Tracked by URL rather than as booleans so viewing a dependency resets both without an effect.
  const [loadedBannerUrl, setLoadedBannerUrl] = useState<string | null>(null)
  const [failedIconUrl, setFailedIconUrl] = useState<string | null>(null)
  const hasBanner = loadedBannerUrl === bannerUrl
  const hasIcon = failedIconUrl !== iconUrl

  return (
    <div className="shrink-0">
      {/* Collapsed until the image resolves, so addons without a banner never expand and snap back. */}
      <div
        className={cn(
          'relative w-full overflow-hidden transition-[height] duration-300 ease-out',
          hasBanner ? 'h-40' : 'h-0'
        )}
      >
        <img
          className="absolute inset-0 h-full w-full object-cover"
          src={bannerUrl}
          alt={`${manifest.alias} banner`}
          onLoad={() => setLoadedBannerUrl(bannerUrl)}
          onError={() => setLoadedBannerUrl(null)}
        />
        <div className="absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-background via-background/70 to-transparent" />
      </div>

      <div
        className={cn(
          'relative z-10 ml-6 w-fit rounded-xl transition-[margin] duration-300 ease-out',
          hasBanner ? '-mt-8 ring-2 ring-background' : 'mt-6'
        )}
      >
        <div className="overflow-hidden rounded-xl ring-1 ring-primary/20">
          <Icon
            manifest={manifest}
            iconUrl={iconUrl}
            hasIcon={hasIcon}
            onIconError={() => setFailedIconUrl(iconUrl)}
            prominent
          />
        </div>
      </div>

      <div className="flex items-start justify-between gap-4 px-6 pt-3 pb-4">
        <div className="min-w-0">
          <DialogTitle className="truncate text-2xl font-semibold">{manifest.alias}</DialogTitle>
          <p className="mt-1 truncate text-sm text-muted-foreground">by {manifest.author}</p>
          {manifest.tags.length > 0 && (
            <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
              {manifest.tags.map(tag => (
                <span
                  key={tag}
                  className="rounded-full bg-muted/60 px-2.5 py-0.5 text-[11px] font-medium text-muted-foreground ring-1 ring-inset ring-border/60"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-1.5">
          {release && (
            <span className="rounded-md bg-primary/10 px-2.5 py-1 font-mono text-xs text-primary ring-1 ring-inset ring-primary/20">
              {release.tag_name}
            </span>
          )}
          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-muted-foreground hover:text-primary"
                  onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}`)}
                  aria-label="View code on GitHub"
                >
                  <GithubIcon className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>View code</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-muted-foreground hover:text-primary"
                  onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}/issues/new`)}
                  aria-label="Report an issue"
                >
                  <BugIcon className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>Report issue</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
    </div>
  )
}
