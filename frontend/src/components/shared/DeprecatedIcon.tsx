import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { type Deprecation, deprecationMessage } from '@/lib/deprecation'
import { cn } from '@/lib/utils'

interface DeprecatedIconProps {
  deprecation: Deprecation | null
  /** Render as a labelled chip instead of a bare mark. */
  chip?: boolean
  /** Use a smaller bare mark. */
  compact?: boolean
  className?: string
}

/** Red "!" for an addon its developer deprecated, the reason shows on hover. */
export const DeprecatedIcon = ({
  deprecation,
  chip = false,
  compact = false,
  className,
}: DeprecatedIconProps) => {
  if (!deprecation) return null

  const mark = (
    <span
      aria-hidden
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-full bg-destructive font-bold leading-none text-white',
        chip ? 'size-3 text-[9px]' : compact ? 'size-3.5 text-[10px]' : 'size-4 text-[11px]'
      )}
    >
      !
    </span>
  )

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {/* Clicks pass through to the row, which is where the uninstall action lives. */}
        <span
          className={cn(
            chip
              ? 'inline-flex shrink-0 items-center gap-1 rounded-sm bg-destructive/15 px-1.5 py-0.5 text-[10px] font-semibold leading-none tracking-wider text-destructive'
              : 'inline-flex shrink-0',
            className
          )}
          aria-label="Deprecated, uninstall recommended"
        >
          {mark}
          {chip && 'DEPRECATED'}
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-xs">{deprecationMessage(deprecation)}</TooltipContent>
    </Tooltip>
  )
}
