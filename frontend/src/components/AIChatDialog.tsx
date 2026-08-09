import * as React from 'react'
import { useEffect, useMemo, useRef } from 'react'

import {
  AssistantTurn,
  buildTurns,
  ChatHeader,
  EmptyState,
  MessageInput,
  useAnimationCleanup,
  useAutoScroll,
  useChatLogic,
  useInputFocus,
  useMarkdownSetup,
  UserMessage,
  useWailsLinkHandler,
} from '@/components/chat'
import type { ChatMessageType } from '@/components/chat/types'
import { useStreamingTextReveal } from '@/components/chat/useStreamingTextReveal'
import { Dialog, DialogContent } from '@/components/ui/dialog'

interface AIChatDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const AIChatDialog = ({ open, onOpenChange }: AIChatDialogProps) => {
  const chatContainerRef = useRef<HTMLDivElement>(null)
  const messagesRef = useRef<HTMLDivElement>(null)
  const messageInputRef = useRef<HTMLInputElement>(null)

  const {
    chatHistory,
    isWaitingForResponse,
    remainingLimit,
    messageAnimationStates,
    sendMessage,
    parseMarkdown,
    copyToClipboard,
    cleanupConnection,
    resetConversation,
    setMessageAnimationStates,
  } = useChatLogic()

  useMarkdownSetup()
  useWailsLinkHandler()

  const turns = useMemo(() => buildTurns(chatHistory), [chatHistory])
  const lastAssistantMessage = chatHistory.findLast(
    (item): item is ChatMessageType => item.type === 'message' && item.role === 'assistant'
  )

  const { displayedText, isRevealing, completeReveal } = useStreamingTextReveal(
    lastAssistantMessage?.content ?? '',
    isWaitingForResponse
  )

  const isInputDisabled = isWaitingForResponse
  const activeTurnId = isWaitingForResponse || isRevealing ? lastAssistantMessage?.id : undefined

  useAutoScroll(chatHistory, chatContainerRef, messagesRef)
  useInputFocus(isInputDisabled, open, messageInputRef)
  useAnimationCleanup(messageAnimationStates, setMessageAnimationStates)

  useEffect(() => {
    if (!open) {
      cleanupConnection()
    }
    return () => cleanupConnection()
  }, [open, cleanupConnection])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (isInputDisabled) return

    const inputValue = messageInputRef.current?.value?.trim() || ''
    if (!inputValue) return

    completeReveal()
    sendMessage(inputValue)
    if (messageInputRef.current) {
      messageInputRef.current.value = ''
    }
  }

  const handleStop = () => {
    cleanupConnection()
    completeReveal()
  }

  const handleClose = () => onOpenChange(false)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="w-[calc(100%-2rem)] max-w-[70%] gap-0 overflow-hidden p-0"
        showCloseButton={false}
      >
        <div className="flex h-[650px] max-h-[calc(100vh-4rem)] flex-col lg:h-[80vh]">
          <ChatHeader
            onClose={handleClose}
            onNewConversation={resetConversation}
            canStartNewConversation={chatHistory.length > 0}
          />

          <div
            ref={chatContainerRef}
            className="scrollbar-thin scrollbar-thumb-secondary scrollbar-track-secondary/20 flex-1 overflow-y-auto p-4 sm:p-6"
          >
            <div className="h-full">
              {turns.length === 0 ? (
                <EmptyState />
              ) : (
                <div ref={messagesRef} className="space-y-6">
                  {turns.map(turn => {
                    if (turn.kind === 'user') {
                      return (
                        <UserMessage
                          key={turn.id}
                          message={turn.message}
                          isAnimating={messageAnimationStates.has(turn.id)}
                        />
                      )
                    }

                    const isActive = turn.id === activeTurnId
                    const hasContent = Boolean(turn.message?.content.trim())
                    if (!isActive && !hasContent && turn.toolCalls.length === 0) return null

                    const isRevealingContent = isActive && isRevealing

                    return (
                      <AssistantTurn
                        key={turn.id}
                        turn={turn}
                        isActive={isActive}
                        isAnimating={messageAnimationStates.has(turn.id)}
                        animatingIds={messageAnimationStates}
                        isRevealingContent={isRevealingContent}
                        revealedText={isRevealingContent ? displayedText : undefined}
                        onCopyMessage={copyToClipboard}
                        parseMarkdown={parseMarkdown}
                      />
                    )
                  })}
                </div>
              )}
            </div>
          </div>

          <MessageInput
            ref={messageInputRef}
            isWaitingForResponse={isWaitingForResponse}
            isRevealingResponse={isRevealing}
            remainingLimit={remainingLimit}
            onSubmit={handleSubmit}
            onStop={handleStop}
          />
        </div>
      </DialogContent>
    </Dialog>
  )
}
