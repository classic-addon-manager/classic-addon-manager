import { ChevronDownIcon, WrenchIcon } from 'lucide-react'
import { useState } from 'react'

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { cn } from '@/lib/utils'

import { ToolPill } from './ToolPill'
import type { ToolCallMessageType } from './types'

interface ToolActivityProps {
  toolCalls: ToolCallMessageType[]
  /** While the turn is live the pills stay expanded so progress is visible. */
  isTurnActive: boolean
  animatingIds: Set<string>
}

export const ToolActivity = ({ toolCalls, isTurnActive, animatingIds }: ToolActivityProps) => {
  const [isOpen, setIsOpen] = useState(false)

  if (toolCalls.length === 0) return null

  const pills = (
    <div className="flex flex-col items-start gap-1.5">
      {toolCalls.map(toolCall => (
        <ToolPill
          key={toolCall.id}
          toolCall={toolCall}
          isAnimating={animatingIds.has(toolCall.id)}
        />
      ))}
    </div>
  )

  if (isTurnActive) {
    return pills
  }

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger className="flex cursor-pointer items-center gap-1.5 rounded-full border border-border/60 bg-muted/30 px-2.5 py-1 text-xs text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50">
        <WrenchIcon className="h-3.5 w-3.5 shrink-0" />
        <span>
          Used {toolCalls.length} {toolCalls.length === 1 ? 'tool' : 'tools'}
        </span>
        <ChevronDownIcon
          className={cn('h-3 w-3 shrink-0 transition-transform', isOpen && 'rotate-180')}
        />
      </CollapsibleTrigger>
      <CollapsibleContent className="pt-1.5">{pills}</CollapsibleContent>
    </Collapsible>
  )
}
