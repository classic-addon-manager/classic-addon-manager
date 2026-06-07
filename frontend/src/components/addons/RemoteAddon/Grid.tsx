import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

import { ActionButton } from './ActionButton'
import { Icon } from './Icon'
import { NewBadge } from './NewBadge'
import { Stats } from './Stats'
import { TagPills } from './TagPills'
import type { RemoteAddonItemProps } from './types'
import { useRemoteAddonItem } from './useRemoteAddonItem'
import { WarningIcon } from './WarningIcon'

export const Grid = ({ manifest, installed }: RemoteAddonItemProps) => {
  const { hasIcon, setIcon, isNew, iconUrl, openDialog } = useRemoteAddonItem({
    manifest,
    installed,
  })

  return (
    <Card
      className="relative cursor-pointer gap-0 py-3 shadow-none transition-all hover:border-primary/30 hover:shadow-sm hover:ring-1 hover:ring-primary/10 group bg-muted/30 hover:bg-muted/50"
      onClick={openDialog}
    >
      <div className="absolute right-1.5 top-1.5 z-10">
        <ActionButton installed={installed} compact />
      </div>

      <CardHeader className="items-center space-y-0 px-3 pb-0 pt-1 text-center">
        <div className="relative mx-auto mb-2 shrink-0">
          <Icon
            manifest={manifest}
            iconUrl={iconUrl}
            hasIcon={hasIcon}
            onIconError={() => setIcon(false)}
            prominent
          />
          <NewBadge isNew={isNew} />
        </div>
        <CardTitle className="flex w-full items-center justify-center gap-1 text-sm leading-tight">
          <span className="truncate group-hover:text-primary transition-colors">
            {manifest.alias}
          </span>
          <WarningIcon warning={manifest.warning} />
        </CardTitle>
        <CardDescription className="truncate text-[11px]" title={`Author: ${manifest.author}`}>
          {manifest.author}
        </CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col items-center gap-1.5 px-3 pt-2 pb-1">
        <Stats manifest={manifest} compact />
        <TagPills tags={manifest.tags} className="justify-center" />
      </CardContent>
    </Card>
  )
}
