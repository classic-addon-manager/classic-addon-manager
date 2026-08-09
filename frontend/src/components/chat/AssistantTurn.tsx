import supportDaruAlt from '@/assets/images/support_daru_alt_sm.webp'
import { cn } from '@/lib/utils'

import type { AssistantTurnData } from './chatTurns'
import { LoadingIndicator } from './LoadingIndicator'
import { MessageActions } from './MessageActions'
import { StreamingMarkdown } from './StreamingMarkdown'
import { ToolActivity } from './ToolActivity'

interface AssistantTurnProps {
  turn: AssistantTurnData
  /** True while the turn is still streaming or catching up on its reveal. */
  isActive: boolean
  isAnimating: boolean
  animatingIds: Set<string>
  isRevealingContent: boolean
  revealedText?: string
  onCopyMessage: (content: string) => Promise<boolean>
  parseMarkdown: (content: string) => string
}

export const AssistantTurn = ({
  turn,
  isActive,
  isAnimating,
  animatingIds,
  isRevealingContent,
  revealedText,
  onCopyMessage,
  parseMarkdown,
}: AssistantTurnProps) => {
  const fullContent = turn.message?.content ?? ''
  const visibleContent =
    isRevealingContent && revealedText !== undefined ? revealedText : fullContent
  const hasRunningTool = turn.toolCalls.some(toolCall => toolCall.status === 'running')

  return (
    <div
      className={cn(
        'group flex gap-3',
        isAnimating && 'animate-in fade-in-0 slide-in-from-bottom-2 duration-600'
      )}
    >
      <div className="flex w-9 shrink-0 flex-col items-center">
        <div className="flex h-9 w-9 items-center justify-center overflow-hidden rounded-full bg-primary/10">
          <img src={supportDaruAlt} alt="Daru Assistant" className="h-full w-full object-cover" />
        </div>
        <div className="mt-2 w-px flex-1 bg-border/60" aria-hidden="true" />
      </div>

      <div className="flex min-w-0 flex-1 flex-col gap-2 pb-2">
        <ToolActivity
          toolCalls={turn.toolCalls}
          isTurnActive={isActive}
          animatingIds={animatingIds}
        />

        {isActive && !visibleContent.trim() && !hasRunningTool && <LoadingIndicator />}

        {turn.message && visibleContent.trim() && (
          <div className="chat-prose text-sm">
            <StreamingMarkdown
              content={visibleContent}
              messageId={turn.message.id}
              animateLatestWord={isRevealingContent}
              parseMarkdown={parseMarkdown}
            />
          </div>
        )}

        {!isActive && fullContent.trim() && (
          <MessageActions content={fullContent} onCopyMessage={onCopyMessage} />
        )}
      </div>
    </div>
  )
}
