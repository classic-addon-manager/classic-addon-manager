import { SendIcon, SquareIcon } from 'lucide-react'
import { forwardRef } from 'react'
import * as React from 'react'

import { Button } from '@/components/ui/button'

interface MessageInputProps {
  isWaitingForResponse: boolean
  isRevealingResponse: boolean
  remainingLimit: number
  onSubmit: (e: React.FormEvent) => void
  onStop: () => void
}

export const MessageInput = forwardRef<HTMLInputElement, MessageInputProps>(
  ({ isWaitingForResponse, isRevealingResponse, remainingLimit, onSubmit, onStop }, ref) => {
    const isInputDisabled = isWaitingForResponse
    const statusMessage = isWaitingForResponse
      ? 'Daru is thinking...'
      : isRevealingResponse
        ? 'Daru is responding...'
        : 'Darus are known for their wisdom, but sometimes even they make mistakes.'

    return (
      <>
        {remainingLimit > 0 && (
          <div className="flex justify-end px-4 pb-2">
            <div className="flex items-center gap-1 rounded-full bg-muted/20 px-2 py-0.5 text-xs text-muted-foreground/60">
              <span>{remainingLimit} messages remaining</span>
            </div>
          </div>
        )}

        <div className="border-t border-border/40 px-4 pt-3 pb-4 sm:px-6 sm:pt-4">
          <form className="flex items-center gap-2" onSubmit={onSubmit}>
            <div className="relative flex-1">
              <input
                ref={ref}
                type="text"
                placeholder={
                  isWaitingForResponse
                    ? statusMessage
                    : "Ask your question, the Darus won't judge..."
                }
                disabled={isInputDisabled}
                autoFocus
                className="w-full rounded-full border bg-muted/30 px-4 py-2.5 pr-16 text-sm focus:ring-2 focus:ring-primary/50 focus:outline-none disabled:opacity-50"
              />

              {isWaitingForResponse ? (
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  onClick={onStop}
                  aria-label="Stop generating"
                  title="Stop generating"
                  className="absolute top-1/2 right-1.5 -translate-y-1/2 transform rounded-full p-1.5"
                >
                  <SquareIcon className="h-3.5 w-3.5 fill-current text-primary" />
                </Button>
              ) : (
                <Button
                  type="submit"
                  size="icon"
                  variant="ghost"
                  aria-label="Send message"
                  title="Send message"
                  className="absolute top-1/2 right-1.5 -translate-y-1/2 transform rounded-full p-1.5"
                >
                  <SendIcon className="h-[18px] w-[18px] text-primary" />
                </Button>
              )}
            </div>
          </form>
          <div className="mt-2 flex min-h-[24px] items-center justify-center">
            <p className="text-center text-xs text-muted-foreground/80">{statusMessage}</p>
          </div>
        </div>
      </>
    )
  }
)

MessageInput.displayName = 'MessageInput'
