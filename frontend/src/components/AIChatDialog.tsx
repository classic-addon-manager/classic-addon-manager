import { useEffect, useRef } from 'react'
import * as React from 'react'

import {
  ChatHeader,
  ChatMessage,
  EmptyState,
  LoadingIndicator,
  MessageInput,
  useAnimationCleanup,
  useAutoScroll,
  useChatLogic,
  useInputFocus,
  useMarkdownSetup,
  useWailsLinkHandler,
} from '@/components/chat'
import { Dialog, DialogContent } from '@/components/ui/dialog'

interface AIChatDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const AIChatDialog = ({ open, onOpenChange }: AIChatDialogProps) => {
  // Refs
  const chatContainerRef = useRef<HTMLDivElement>(null)
  const messageInputRef = useRef<HTMLInputElement>(null)

  // Custom hooks
  const {
    chatHistory,
    isWaitingForResponse,
    remainingLimit,
    messageAnimationStates,
    sendMessage,
    parseMarkdown,
    copyToClipboard,
    cleanupConnection,
    setMessageAnimationStates,
  } = useChatLogic()

  // Setup hooks
  useMarkdownSetup()
  useWailsLinkHandler()
  useAutoScroll(chatHistory, chatContainerRef)
  useInputFocus(isWaitingForResponse, open, messageInputRef)
  useAnimationCleanup(messageAnimationStates, setMessageAnimationStates)

  // Cleanup on close/unmount
  useEffect(() => {
    if (!open) {
      cleanupConnection()
    }
    return () => cleanupConnection()
  }, [open, cleanupConnection])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const inputValue = messageInputRef.current?.value?.trim() || ''
    if (inputValue) {
      sendMessage(inputValue)
      if (messageInputRef.current) {
        messageInputRef.current.value = ''
      }
    }
  }

  const handleClose = () => onOpenChange(false)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed left-[50%] top-[50%] z-50 w-[calc(100%-2rem)] max-w-[70%] translate-x-[-50%] translate-y-[-50%] gap-0 rounded-lg border bg-background shadow-lg duration-200 p-0"
        showCloseButton={false}
      >
        <div className="flex max-h-[calc(100vh-4rem)] h-[650px] lg:h-[80vh] flex-col">
          {/* Header */}
          <ChatHeader onClose={handleClose} />

          {/* Chat Area */}
          <div
            ref={chatContainerRef}
            className="flex-1 overflow-y-auto p-4 sm:p-6 scrollbar-thin scrollbar-thumb-secondary scrollbar-track-secondary/20"
          >
            <div className="h-full">
              {chatHistory.length === 0 ? (
                <EmptyState />
              ) : (
                <div className="space-y-6">
                  {chatHistory.map(message => {
                    if (message.role === 'user') {
                      return (
                        <ChatMessage
                          key={message.id}
                          message={message}
                          isAnimating={messageAnimationStates.has(message.id)}
                          onCopyMessage={copyToClipboard}
                          parseMarkdown={parseMarkdown}
                        />
                      )
                    }
                    if (message.role === 'assistant' && message.content.trim()) {
                      return (
                        <ChatMessage
                          key={message.id}
                          message={message}
                          isAnimating={messageAnimationStates.has(message.id)}
                          onCopyMessage={copyToClipboard}
                          parseMarkdown={parseMarkdown}
                        />
                      )
                    }
                    return null
                  })}

                  {isWaitingForResponse && <LoadingIndicator />}
                </div>
              )}
            </div>
          </div>

          {/* Input Area */}
          <MessageInput
            ref={messageInputRef}
            isWaitingForResponse={isWaitingForResponse}
            remainingLimit={remainingLimit}
            onSubmit={handleSubmit}
          />
        </div>
      </DialogContent>
    </Dialog>
  )
}
