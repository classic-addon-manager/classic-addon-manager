import { WrenchIcon } from 'lucide-react'
import * as React from 'react'
import { useEffect, useRef } from 'react'

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

const TOOL_CALL_MESSAGES: Record<string, string> = {
  list_addons: 'Getting addons...',
  get_latest_release: 'Checking the latest release...',
  get_addon_readme: 'Opening the addon README...',
  get_addon_manager_documentation: 'Fetching manager instructions...',
  get_addon_details: 'Gathering addon details...',
  search_archeage_wiki: 'Searching the ArcheAge Classic wiki...',
  get_archeage_wiki_page_content: 'Reading the wiki page...',
}

const getToolCallMessage = (action: string) => {
  if (action in TOOL_CALL_MESSAGES) {
    return TOOL_CALL_MESSAGES[action]
  }
  return 'Unknown tool call'
}

interface ToolCallEntryProps {
  action: string
  isAnimating: boolean
}

const ToolCallEntry = ({ action, isAnimating }: ToolCallEntryProps) => (
  <div
    className={`flex justify-center px-4 ${
      isAnimating ? 'animate-in fade-in-0 slide-in-from-bottom-2 duration-600' : ''
    }`}
  >
    <div className="flex items-center gap-3 rounded-md border border-dashed border-border bg-muted/50 px-4 py-2 text-sm text-muted-foreground">
      <span className="flex h-7 w-7 items-center justify-center rounded-full bg-primary/10 text-primary">
        <WrenchIcon className="h-4 w-4" />
      </span>
      <div className="flex flex-col">
        <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground/80">
          Tool call
        </span>
        <span className="text-sm text-foreground">{getToolCallMessage(action)}</span>
      </div>
    </div>
  </div>
)

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
                  {chatHistory.map(historyItem => {
                    if (historyItem.type === 'tool_call') {
                      return (
                        <ToolCallEntry
                          key={historyItem.id}
                          action={historyItem.action}
                          isAnimating={messageAnimationStates.has(historyItem.id)}
                        />
                      )
                    }
                    if (historyItem.type === 'message' && historyItem.role === 'user') {
                      return (
                        <ChatMessage
                          key={historyItem.id}
                          message={historyItem}
                          isAnimating={messageAnimationStates.has(historyItem.id)}
                          onCopyMessage={copyToClipboard}
                          parseMarkdown={parseMarkdown}
                        />
                      )
                    }
                    if (
                      historyItem.type === 'message' &&
                      historyItem.role === 'assistant' &&
                      historyItem.content.trim()
                    ) {
                      return (
                        <ChatMessage
                          key={historyItem.id}
                          message={historyItem}
                          isAnimating={messageAnimationStates.has(historyItem.id)}
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
