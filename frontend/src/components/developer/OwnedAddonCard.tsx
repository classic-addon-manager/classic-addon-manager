import { Browser } from '@wailsio/runtime'
import { BlocksIcon, DownloadIcon, GithubIcon, HeartIcon } from 'lucide-react'
import { useState } from 'react'

import { TagPills } from '@/components/addons/RemoteAddon/TagPills'
import { WarningIcon } from '@/components/addons/RemoteAddon/WarningIcon'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { formatToLocalDate } from '@/lib/utils'

export const OwnedAddonCard = ({ addon }: { addon: OwnedAddon }) => {
  const [hasIcon, setHasIcon] = useState(true)
  const showIcon = addon.branch !== null && hasIcon
  const iconUrl =
    addon.branch !== null
      ? `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`
      : ''

  return (
    <div className="rounded-xl border border-border/50 bg-muted/30 p-4">
      <div className="flex items-start gap-3">
        {showIcon ? (
          <img
            className="h-10 w-10 rounded-lg object-cover border border-border/50 shadow-xs"
            src={iconUrl}
            alt={`${addon.alias} icon`}
            loading="lazy"
            onError={() => setHasIcon(false)}
          />
        ) : (
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-background border border-border/50 shadow-xs">
            <BlocksIcon className="h-5 w-5 opacity-40 stroke-[1.5]" />
          </div>
        )}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="truncate font-medium">{addon.alias}</span>
            <WarningIcon warning={addon.warning} />
          </div>
          {addon.author !== '' && <p className="text-sm text-muted-foreground">{addon.author}</p>}
        </div>
        {addon.repo !== '' && (
          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="ml-auto h-8 w-8 text-muted-foreground hover:text-primary"
                  onClick={() => void Browser.OpenURL(`https://github.com/${addon.repo}`)}
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
        )}
      </div>
      <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
        <div className="flex items-center gap-0.5" title={`${addon.downloads} downloads`}>
          <DownloadIcon className="h-3 w-3" /> {addon.downloads}
        </div>
        {addon.likePercentage !== null && (
          <div className="flex items-center gap-0.5" title={`${addon.likePercentage}% likes`}>
            <HeartIcon className="h-3 w-3" /> {addon.likePercentage}%
          </div>
        )}
      </div>
      {addon.description !== '' && (
        <p className="mt-2 line-clamp-2 text-sm text-muted-foreground">{addon.description}</p>
      )}
      <TagPills tags={addon.tags} maxTags={4} className="mt-2" />
      {addon.addedAt !== null && (
        <p className="mt-2 text-xs text-muted-foreground">{formatToLocalDate(addon.addedAt)}</p>
      )}
    </div>
  )
}
