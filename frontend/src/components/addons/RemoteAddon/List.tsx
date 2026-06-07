import { ActionButton } from './ActionButton'
import { Icon } from './Icon'
import { NewBadge } from './NewBadge'
import { Stats } from './Stats'
import { TagPills } from './TagPills'
import type { RemoteAddonItemProps } from './types'
import { useRemoteAddonItem } from './useRemoteAddonItem'
import { WarningIcon } from './WarningIcon'

export const List = ({ manifest, installed }: RemoteAddonItemProps) => {
  const { hasIcon, setIcon, isNew, iconUrl, openDialog } = useRemoteAddonItem({
    manifest,
    installed,
  })

  return (
    <div
      className="grid grid-cols-12 items-center gap-2 bg-muted/30 hover:bg-muted/50 h-16 w-full rounded-xl cursor-pointer transition-all px-4 py-2 border border-border/50 hover:border-primary/30 hover:shadow-sm hover:ring-1 hover:ring-primary/10 group"
      onClick={openDialog}
    >
      <div className="flex items-center gap-3 col-span-5">
        <div className="relative shrink-0">
          <Icon
            manifest={manifest}
            iconUrl={iconUrl}
            hasIcon={hasIcon}
            onIconError={() => setIcon(false)}
          />
          <NewBadge isNew={isNew} />
        </div>
        <div className="flex flex-col overflow-hidden">
          <div className="flex items-center gap-1.5 overflow-hidden">
            <span className="text-foreground font-medium truncate group-hover:text-primary transition-colors">
              {manifest.alias}
            </span>
            <WarningIcon warning={manifest.warning} />
          </div>
          <div className="mt-0.5">
            <Stats manifest={manifest} />
          </div>
        </div>
      </div>

      <div
        className="text-sm text-muted-foreground truncate text-center col-span-3"
        title={`Author: ${manifest.author}`}
      >
        {manifest.author}
      </div>

      <div className="col-span-3 flex justify-end">
        <TagPills tags={manifest.tags} maxTags={4} />
      </div>

      <div className="flex justify-end col-span-1">
        <ActionButton installed={installed} />
      </div>
    </div>
  )
}
