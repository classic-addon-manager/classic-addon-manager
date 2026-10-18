import { ArrowRightIcon, CheckIcon, Trash2Icon } from 'lucide-react'

interface ActionButtonProps {
  installed: boolean
  /** A deprecated addon only offers removal, which the details dialog performs. */
  deprecated?: boolean
  compact?: boolean
}

/** Expands to a "View" pill when the surrounding `group` is hovered. */
export const ActionButton = ({
  installed,
  deprecated = false,
  compact = false,
}: ActionButtonProps) => {
  const heightClass = compact ? 'h-6' : 'h-8'
  const iconClass = compact ? 'h-3.5 w-3.5' : 'h-4 w-4'

  if (installed && deprecated) {
    return (
      <span
        className={`inline-flex items-center font-semibold text-destructive ${heightClass} ${compact ? 'gap-1 text-[11px]' : 'gap-1.5 px-2 text-xs'}`}
      >
        <Trash2Icon className={iconClass} />
        Uninstall
      </span>
    )
  }

  if (installed) {
    return (
      <span
        className={`inline-flex items-center font-medium text-green-500 ${heightClass} ${compact ? 'gap-1 text-[11px]' : 'gap-1.5 px-2 text-xs'}`}
      >
        <CheckIcon className={iconClass} />
        Installed
      </span>
    )
  }

  return (
    <button
      type="button"
      aria-label="View details"
      className={`group/view inline-flex cursor-pointer items-center font-semibold text-muted-foreground/70 outline-none transition-all group-hover:bg-primary group-hover:text-primary-foreground focus-visible:bg-primary focus-visible:text-primary-foreground ${heightClass} ${compact ? 'rounded-md px-1 text-[11px] group-hover:px-2 focus-visible:px-2' : 'rounded-lg px-2 text-xs group-hover:px-3 focus-visible:px-3'}`}
    >
      <span className="max-w-0 overflow-hidden opacity-0 transition-all group-hover:mr-1.5 group-hover:max-w-12 group-hover:opacity-100 group-focus-visible/view:mr-1.5 group-focus-visible/view:max-w-12 group-focus-visible/view:opacity-100">
        View
      </span>
      <ArrowRightIcon className={iconClass} />
    </button>
  )
}
