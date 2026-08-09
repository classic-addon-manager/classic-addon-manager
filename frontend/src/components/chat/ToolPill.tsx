import { CheckIcon, Loader2Icon } from 'lucide-react'

import { cn } from '@/lib/utils'

import { getToolMeta } from './toolCatalog'
import type { ToolCallMessageType } from './types'

interface ToolPillProps {
  toolCall: ToolCallMessageType
  isAnimating?: boolean
}

export const ToolPill = ({ toolCall, isAnimating = false }: ToolPillProps) => {
  const { running, done, icon: Icon } = getToolMeta(toolCall.action)
  const isRunning = toolCall.status === 'running'

  return (
    <div
      className={cn(
        'flex w-fit max-w-full items-center gap-2 rounded-full border px-2.5 py-1 text-xs',
        isRunning
          ? 'border-primary/30 bg-primary/5 text-foreground'
          : 'border-border/60 bg-muted/40 text-muted-foreground',
        isAnimating && 'animate-in fade-in-0 slide-in-from-bottom-1 duration-500'
      )}
    >
      <Icon className="h-3.5 w-3.5 shrink-0 text-primary/80" />
      <span className="truncate">{isRunning ? running : done}</span>
      {isRunning ? (
        <Loader2Icon className="h-3 w-3 shrink-0 animate-spin text-primary" />
      ) : (
        <CheckIcon className="h-3 w-3 shrink-0 text-primary/70" />
      )}
    </div>
  )
}
