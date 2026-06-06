import { TriangleAlertIcon } from 'lucide-react'

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

interface WarningIconProps {
  warning?: string | null
}

export const WarningIcon = ({ warning }: WarningIconProps) => {
  if (!warning) return null

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          onClick={e => e.stopPropagation()}
          className="shrink-0 text-amber-500"
          aria-label="Warning"
        >
          <TriangleAlertIcon className="h-4 w-4" />
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-xs">{warning}</TooltipContent>
    </Tooltip>
  )
}
