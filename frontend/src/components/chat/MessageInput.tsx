import { forwardRef } from 'react'
import * as React from 'react'

import { Button } from '@/components/ui/button'

interface MessageInputProps {
  isWaitingForResponse: boolean
  remainingLimit: number
  onSubmit: (e: React.FormEvent) => void
}

export const MessageInput = forwardRef<HTMLInputElement, MessageInputProps>(
  ({ isWaitingForResponse, remainingLimit, onSubmit }, ref) => {
    return (
      <>
        {remainingLimit > 0 && (
          <div className="flex justify-end px-4 pb-2">
            <div className="text-xs text-muted-foreground/60 flex items-center gap-1 px-2 py-0.5 rounded-full bg-muted/20">
              <span>{remainingLimit} messages remaining</span>
            </div>
          </div>
        )}

        <div className="border-t bg-background pt-3 sm:pt-4 px-4 sm:px-6 pb-4">
          <form className="flex items-center gap-2" onSubmit={onSubmit}>
            <div className="relative flex-1">
              <input
                ref={ref}
                type="text"
                placeholder={
                  isWaitingForResponse
                    ? 'Daru is thinking...'
                    : "Ask your question, the Darus won't judge..."
                }
                disabled={isWaitingForResponse}
                autoFocus
                className="w-full rounded-full border bg-muted/30 px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 pr-16 disabled:opacity-50"
              />

              <Button
                type="submit"
                size="icon"
                className="absolute right-1.5 top-1/2 -translate-y-1/2 transform rounded-full p-1.5"
                variant="ghost"
                disabled={isWaitingForResponse}
              >
                {isWaitingForResponse ? (
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>
                ) : (
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    className="text-primary"
                  >
                    <path d="m22 2-7 20-4-9-9-4Z" />
                    <path d="M22 2 11 13" />
                  </svg>
                )}
              </Button>
            </div>
          </form>
          <div className="flex items-center justify-center mt-2 min-h-[24px]">
            <p className="text-center text-xs text-muted-foreground/80">
              {isWaitingForResponse
                ? 'Daru is thinking...'
                : 'Darus are known for their wisdom, but sometimes even they make mistakes.'}
            </p>
          </div>
        </div>
      </>
    )
  }
)

MessageInput.displayName = 'MessageInput'
