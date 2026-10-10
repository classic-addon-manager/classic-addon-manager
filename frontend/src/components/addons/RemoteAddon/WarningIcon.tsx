import { TriangleAlertIcon } from 'lucide-react'

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

interface WarningIconProps {
  warning?: string | null
  /** Render as a labelled chip instead of a bare icon. */
  chip?: boolean
  /** Use a smaller bare icon. */
  compact?: boolean
}

export const WarningIcon = ({ warning, chip = false, compact = false }: WarningIconProps) => {
  if (!warning) return null

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          onClick={e => e.stopPropagation()}
          className={
            chip
              ? 'inline-flex shrink-0 items-center gap-1 rounded-sm bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-semibold leading-none tracking-wider text-amber-500'
              : 'shrink-0 text-amber-500'
          }
          aria-label="Warning"
        >
          <TriangleAlertIcon
            className={chip ? 'h-2.5 w-2.5' : compact ? 'h-3.5 w-3.5' : 'h-4 w-4'}
          />
          {chip && 'CAUTION'}
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-xs">{warning}</TooltipContent>
    </Tooltip>
  )
}
