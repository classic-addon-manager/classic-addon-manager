import { Browser } from '@wailsio/runtime'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { useAtomValue } from 'jotai'
import { useEffect, useRef, useState } from 'react'

import supportDaru from '@/assets/images/support_daru.webp'
import supportDaruAlt from '@/assets/images/support_daru_alt_sm.webp'
import { versionAtom } from '@/atoms/applicationAtoms'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast'
import { useUserStore } from '@/stores/userStore'

interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  id: string
}

interface AIChatDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const AIChatDialog = ({ open, onOpenChange }: AIChatDialogProps) => {
  const { user, token } = useUserStore()
  const version = useAtomValue(versionAtom)

  // State
  const [chatMessage, setChatMessage] = useState('')
  const [chatHistory, setChatHistory] = useState<ChatMessage[]>([])
  const [isWaitingForResponse, setIsWaitingForResponse] = useState(false)
  const [conversationId, setConversationId] = useState<string | null>(null)
  const [remainingLimit, setRemainingLimit] = useState(0)

  // Refs
  const chatContainerRef = useRef<HTMLDivElement>(null)
  const messageInputRef = useRef<HTMLInputElement>(null)
  const activeEventSourceRef = useRef<EventSource | null>(null)
  const currentAssistantContentRef = useRef<string>('')

  // Animation tracking
  const [messageAnimationStates, setMessageAnimationStates] = useState<Set<string>>(new Set())

  // Initialize markdown and DOMPurify
  useEffect(() => {
    marked.setOptions({ gfm: true, breaks: true })

    DOMPurify.addHook('afterSanitizeAttributes', function (node) {
      if (node.tagName === 'A' && node.hasAttribute('href')) {
        const url = node.getAttribute('href')
        if (url) {
          node.setAttribute('href', 'javascript:void(0)')
          node.setAttribute('data-url', url)
          node.classList.add('wails-link')
          node.setAttribute(
            'onclick',
            `event.preventDefault(); window.runtime.Browser.OpenURL('${url.replace(/'/g, "\\'")}');`
          )
        }
      }
    })

    return () => DOMPurify.removeHook('afterSanitizeAttributes')
  }, [])

  // Auto-scroll to bottom
  useEffect(() => {
    if (chatHistory.length > 0 && chatContainerRef.current) {
      setTimeout(() => {
        chatContainerRef.current?.scrollTo({
          top: chatContainerRef.current.scrollHeight,
          behavior: 'smooth',
        })
      }, 50)
    }
  }, [chatHistory])

  // Focus input when not waiting
  useEffect(() => {
    if (!isWaitingForResponse && messageInputRef.current && open) {
      setTimeout(() => messageInputRef.current?.focus(), 0)
    }
  }, [isWaitingForResponse, open])

  // Clean up animations after delay
  useEffect(() => {
    if (messageAnimationStates.size > 0) {
      const timer = setTimeout(() => {
        setMessageAnimationStates(new Set())
      }, 1000)
      return () => clearTimeout(timer)
    }
  }, [messageAnimationStates])

  // Cleanup SSE on close/unmount
  useEffect(() => {
    if (!open && activeEventSourceRef.current) {
      cleanupConnection()
    }
    return () => cleanupConnection()
  }, [open])

  // Handle clicks on wails links
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (e.target instanceof HTMLElement) {
        const target = e.target
        if (!target.classList.contains('wails-link')) {
          return
        }
        const url = target.getAttribute('data-url')
        if (url) Browser.OpenURL(url)
      }
    }

    window.addEventListener('click', handleClick)
    return () => window.removeEventListener('click', handleClick)
  }, [])

  const cleanupConnection = () => {
    if (activeEventSourceRef.current) {
      activeEventSourceRef.current.close()
      activeEventSourceRef.current = null
    }
    setIsWaitingForResponse(false)
  }

  const parseMarkdown = (content: string) => {
    return DOMPurify.sanitize(marked.parse(content, { async: false }) as string, {
      ADD_ATTR: ['data-url', 'onclick'],
    })
  }

  const generateMessageId = () => {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  }

  const sendMessage = async () => {
    if (!chatMessage.trim() || isWaitingForResponse) return

    cleanupConnection()

    // Add user message
    const userMessageId = generateMessageId()
    const userMessage = chatMessage
    setChatMessage('')

    setChatHistory(prev => [...prev, { role: 'user', content: userMessage, id: userMessageId }])
    setMessageAnimationStates(prev => new Set([...prev, userMessageId]))

    // Add empty assistant message for streaming
    const assistantMessageId = generateMessageId()
    currentAssistantContentRef.current = '' // Reset content ref
    setChatHistory(prev => [...prev, { role: 'assistant', content: '', id: assistantMessageId }])
    setMessageAnimationStates(prev => new Set([...prev, assistantMessageId]))

    setIsWaitingForResponse(true)

    // Timeout for slow response
    const timeoutId = setTimeout(() => {
      if (isWaitingForResponse) {
        setChatHistory(prev => {
          const newHistory = [...prev]
          const assistantMessageIndex = newHistory.findIndex(msg => msg.id === assistantMessageId)
          if (assistantMessageIndex !== -1 && !newHistory[assistantMessageIndex].content) {
            newHistory[assistantMessageIndex].content =
              'Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸'
          }
          return newHistory
        })
      }
    }, 10000)

    try {
      const params = new URLSearchParams({
        p: userMessage,
        client: version,
        ...(conversationId && { conversation_id: conversationId }),
        ...(token && { token }),
      })

      const eventSource = new EventSource(`https://aac.gaijin.dev/ai/chat/stream?${params}`)
      activeEventSourceRef.current = eventSource

      eventSource.onmessage = event => {
        clearTimeout(timeoutId)

        try {
          const data = JSON.parse(event.data)

          if (data.type === 'message') {
            // Append to ref first to prevent race conditions
            currentAssistantContentRef.current += data.data || ''

            setChatHistory(prev => {
              const newHistory = [...prev]
              const assistantMessageIndex = newHistory.findIndex(
                msg => msg.id === assistantMessageId
              )
              if (assistantMessageIndex !== -1) {
                newHistory[assistantMessageIndex].content = currentAssistantContentRef.current
              }
              return newHistory
            })

            if (data.conversation_id) setConversationId(data.conversation_id)
            if (data.remaining_limit !== undefined) setRemainingLimit(data.remaining_limit)
          } else if (data.type === 'complete') {
            if (data.conversation_id) setConversationId(data.conversation_id)
            if (data.remaining_limit !== undefined) setRemainingLimit(data.remaining_limit)
            cleanupConnection()
          } else if (data.type === 'error') {
            currentAssistantContentRef.current = `Sorry, I encountered an error: ${data.message || 'Server error'}`
            setChatHistory(prev => {
              const newHistory = [...prev]
              const assistantMessageIndex = newHistory.findIndex(
                msg => msg.id === assistantMessageId
              )
              if (assistantMessageIndex !== -1) {
                newHistory[assistantMessageIndex].content = currentAssistantContentRef.current
              }
              return newHistory
            })
            cleanupConnection()
          }
        } catch (parseError) {
          // Treat as raw content chunk - append to ref first
          currentAssistantContentRef.current += event.data
          setChatHistory(prev => {
            const newHistory = [...prev]
            const assistantMessageIndex = newHistory.findIndex(msg => msg.id === assistantMessageId)
            if (assistantMessageIndex !== -1) {
              newHistory[assistantMessageIndex].content = currentAssistantContentRef.current
            }
            return newHistory
          })
        }
      }

      eventSource.onerror = () => {
        clearTimeout(timeoutId)
        if (!currentAssistantContentRef.current) {
          currentAssistantContentRef.current =
            'Sorry, I encountered an error processing your request.'
        }
        setChatHistory(prev => {
          const newHistory = [...prev]
          const assistantMessageIndex = newHistory.findIndex(msg => msg.id === assistantMessageId)
          if (assistantMessageIndex !== -1) {
            newHistory[assistantMessageIndex].content = currentAssistantContentRef.current
          }
          return newHistory
        })
        cleanupConnection()
      }
    } catch (error) {
      clearTimeout(timeoutId)
      currentAssistantContentRef.current = 'Sorry, I encountered an error processing your request.'
      setChatHistory(prev => {
        const newHistory = [...prev]
        const assistantMessageIndex = newHistory.findIndex(msg => msg.id === assistantMessageId)
        if (assistantMessageIndex !== -1) {
          newHistory[assistantMessageIndex].content = currentAssistantContentRef.current
        }
        return newHistory
      })
      cleanupConnection()
    }
  }

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      toast({
        title: 'Success',
        description: 'Copied to clipboard',
      })
    } catch {
      toast({
        title: 'Error',
        description: 'Failed to copy text',
      })
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed left-[50%] top-[50%] z-50 w-[calc(100%-2rem)] max-w-[70%] translate-x-[-50%] translate-y-[-50%] gap-0 rounded-lg border bg-background shadow-lg duration-200 p-0"
        showCloseButton={false}
      >
        <div className="flex max-h-[calc(100vh-4rem)] h-[650px] lg:h-[80vh] flex-col">
          {/* Header */}
          <div className="border-b border-border/40 px-4 sm:px-6 py-4">
            <div className="flex items-center justify-between">
              <div>
                <DialogTitle className="text-lg font-medium leading-none">
                  Daru Informational Network
                </DialogTitle>
                <DialogDescription className="mt-2 text-sm text-muted-foreground">
                  The Darus have information, if you have coin.
                </DialogDescription>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => onOpenChange(false)}
                className="h-6 w-6"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="m18 6-12 12" />
                  <path d="m6 6 12 12" />
                </svg>
              </Button>
            </div>
          </div>

          {/* Chat Area */}
          <div
            ref={chatContainerRef}
            className="flex-1 overflow-y-auto p-4 sm:p-6 scrollbar-thin scrollbar-thumb-secondary scrollbar-track-secondary/20"
          >
            <div className="h-full">
              {chatHistory.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center space-y-6 text-center px-4">
                  <div className="space-y-6 max-h-full">
                    <img
                      src={supportDaru}
                      alt="Support Daru"
                      className="w-[35vh] h-[35vh] max-w-[280px] max-h-[280px] min-w-[160px] min-h-[160px] object-contain mx-auto drop-shadow-lg hover:scale-105 transition-transform duration-300"
                    />
                    <div>
                      <h3 className="text-xl font-semibold text-primary mb-2">
                        Welcome to Daru's Help Desk!
                      </h3>
                      <p className="text-sm text-muted-foreground max-w-md mx-auto leading-relaxed">
                        I'm your friendly Daru assistant, ready to help with all your addon needs.
                        Feel free to ask me anything!
                      </p>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="space-y-6">
                  {chatHistory.map(message => {
                    if (
                      message.role === 'user' ||
                      (message.role === 'assistant' && message.content.trim())
                    ) {
                      return (
                        <div
                          key={message.id}
                          className={`flex items-start gap-3 px-4 ${
                            message.role === 'user' ? 'justify-end' : ''
                          } ${
                            messageAnimationStates.has(message.id)
                              ? 'animate-in fade-in-0 slide-in-from-bottom-2 duration-600'
                              : ''
                          }`}
                        >
                          {message.role === 'assistant' && (
                            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                              <img
                                src={supportDaruAlt}
                                alt="Daru Assistant"
                                className="w-full h-full object-cover"
                              />
                            </div>
                          )}

                          <div
                            className={`flex flex-col gap-2 ${
                              message.role === 'assistant'
                                ? 'max-w-[calc(100%-3.5rem)] w-full'
                                : 'max-w-[80%]'
                            }`}
                          >
                            <div
                              className={`relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message group ${
                                message.role === 'assistant'
                                  ? 'assistant-message bg-muted/50'
                                  : 'bg-primary text-black'
                              }`}
                            >
                              {message.role === 'assistant' ? (
                                <>
                                  <div
                                    dangerouslySetInnerHTML={{
                                      __html: parseMarkdown(message.content),
                                    }}
                                  />
                                  <button
                                    className="copy-button absolute top-1 right-1 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-secondary/50 transition-all duration-150 opacity-0 group-hover:opacity-100"
                                    onClick={() => copyToClipboard(message.content)}
                                  >
                                    <svg
                                      xmlns="http://www.w3.org/2000/svg"
                                      width="14"
                                      height="14"
                                      viewBox="0 0 24 24"
                                      fill="none"
                                      stroke="currentColor"
                                      strokeWidth="2"
                                      strokeLinecap="round"
                                      strokeLinejoin="round"
                                    >
                                      <rect width="8" height="4" x="8" y="2" rx="1" ry="1" />
                                      <path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
                                      <path d="M16 4h2a2 2 0 0 1 2 2v4" />
                                      <path d="M21 14H11" />
                                      <path d="m15 10-4 4 4 4" />
                                    </svg>
                                  </button>
                                </>
                              ) : (
                                message.content
                              )}
                            </div>
                          </div>

                          {message.role === 'user' && (
                            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-secondary">
                              <Avatar className="h-full w-full">
                                <AvatarImage
                                  src={`https://cdn.discordapp.com/avatars/${user.discord_id}/${user.avatar}.png`}
                                  alt={user.username}
                                  className="h-full w-full object-cover"
                                />
                                <AvatarFallback className="text-xs">
                                  {user.username.substring(0, 2).toUpperCase()}
                                </AvatarFallback>
                              </Avatar>
                            </div>
                          )}
                        </div>
                      )
                    }
                    return null
                  })}

                  {isWaitingForResponse && (
                    <div className="flex items-start gap-3 px-4">
                      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                        <img
                          src={supportDaruAlt}
                          alt="Daru Assistant"
                          className="w-full h-full object-cover"
                        />
                      </div>
                      <div className="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50 border">
                        <div className="flex gap-1.5 items-center">
                          <span className="text-muted-foreground/70 text-xs mr-2">
                            Daru is thinking
                          </span>
                          <span className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"></span>
                          <span
                            className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
                            style={{ animationDelay: '0.1s' }}
                          ></span>
                          <span
                            className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
                            style={{ animationDelay: '0.2s' }}
                          ></span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          {remainingLimit > 0 && (
            <div className="flex justify-end px-4 pb-2">
              <div className="text-xs text-muted-foreground/60 flex items-center gap-1 px-2 py-0.5 rounded-full bg-muted/20">
                <span>{remainingLimit} messages remaining</span>
              </div>
            </div>
          )}

          {/* Input Area */}
          <div className="border-t bg-background pt-3 sm:pt-4 px-4 sm:px-6 pb-4">
            <form
              className="flex items-center gap-2"
              onSubmit={e => {
                e.preventDefault()
                sendMessage()
              }}
            >
              <div className="relative flex-1">
                <input
                  ref={messageInputRef}
                  type="text"
                  value={chatMessage}
                  onChange={e => setChatMessage(e.target.value)}
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
                  disabled={!chatMessage.trim() || isWaitingForResponse}
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
        </div>
      </DialogContent>
    </Dialog>
  )
}
