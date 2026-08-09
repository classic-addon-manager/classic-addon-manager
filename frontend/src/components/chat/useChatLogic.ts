import { Browser } from '@wailsio/runtime'
import DOMPurify from 'dompurify'
import { useAtomValue } from 'jotai'
import { marked } from 'marked'
import { useCallback, useEffect, useRef, useState } from 'react'

import { versionAtom } from '@/atoms/applicationAtoms'
import { useUserStore } from '@/stores/userStore'

import { copyToClipboard, generateMessageId, parseMarkdown } from './chatUtils'
import type { ChatHistoryItem } from './types'

type TimeoutRef = { current: ReturnType<typeof setTimeout> | null }

const clearTimeoutRef = (ref: TimeoutRef) => {
  if (ref.current) {
    clearTimeout(ref.current)
    ref.current = null
  }
}

/**
 * The server never reports tool completion, so a call is considered done as soon
 * as any later event arrives. Returns the same array when nothing changed so
 * React can skip the re-render.
 */
const markToolCallsDone = (items: ChatHistoryItem[]): ChatHistoryItem[] => {
  let changed = false
  const next = items.map(item => {
    if (item.type !== 'tool_call' || item.status === 'done') return item
    changed = true
    return { ...item, status: 'done' as const }
  })
  return changed ? next : items
}

export const useChatLogic = () => {
  const { token } = useUserStore()
  const version = useAtomValue(versionAtom)

  // State
  const [chatHistory, setChatHistory] = useState<ChatHistoryItem[]>([])
  const [isWaitingForResponse, setIsWaitingForResponse] = useState(false)
  const [conversationId, setConversationId] = useState<string | null>(null)
  const [remainingLimit, setRemainingLimit] = useState(0)
  const [messageAnimationStates, setMessageAnimationStates] = useState<Set<string>>(new Set())

  // Refs
  const activeEventSourceRef = useRef<EventSource | null>(null)
  const currentAssistantContentRef = useRef<string>('')
  const slowResponseTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const cleanupConnection = useCallback(() => {
    if (activeEventSourceRef.current) {
      activeEventSourceRef.current.close()
      activeEventSourceRef.current = null
    }
    clearTimeoutRef(slowResponseTimeoutRef)
    setChatHistory(markToolCallsDone)
    setIsWaitingForResponse(false)
  }, [])

  const resetConversation = useCallback(() => {
    cleanupConnection()
    currentAssistantContentRef.current = ''
    setChatHistory([])
    setConversationId(null)
    setMessageAnimationStates(new Set())
  }, [cleanupConnection])

  // --- Helpers ---

  const finishToolCalls = () => {
    setChatHistory(markToolCallsDone)
  }

  const updateAssistantMessage = (assistantMessageId: string, content: string) => {
    setChatHistory(prev =>
      prev.map(msg =>
        msg.id === assistantMessageId && msg.type === 'message' ? { ...msg, content } : msg
      )
    )
  }

  const syncMetadata = (data: { conversation_id?: string; remaining_limit?: number }) => {
    if (data.conversation_id) setConversationId(data.conversation_id)
    if (data.remaining_limit !== undefined) setRemainingLimit(data.remaining_limit)
  }

  const startSlowResponseTimeout = (assistantMessageId: string) => {
    slowResponseTimeoutRef.current = setTimeout(() => {
      setChatHistory(prev =>
        prev.map(msg =>
          msg.id === assistantMessageId && msg.type === 'message' && !msg.content
            ? {
                ...msg,
                content:
                  'Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸',
              }
            : msg
        )
      )
    }, 10000)
  }

  // --- SSE Event Handlers ---

  const handleMessageChunk = (
    data: { data?: string; conversation_id?: string; remaining_limit?: number },
    assistantMessageId: string
  ) => {
    currentAssistantContentRef.current += data.data || ''
    finishToolCalls()
    updateAssistantMessage(assistantMessageId, currentAssistantContentRef.current)
    syncMetadata(data)
  }

  const handleToolCall = (
    data: { data?: string; conversation_id?: string; remaining_limit?: number },
    assistantMessageId: string
  ) => {
    const toolCallId = generateMessageId()
    const action = typeof data.data === 'string' ? data.data : 'unknown_tool'
    const toolCallEntry = {
      id: toolCallId,
      type: 'tool_call' as const,
      action,
      status: 'running' as const,
    }
    setChatHistory(prev => {
      const settled = markToolCallsDone(prev)
      const found = settled.some(item => item.id === assistantMessageId)
      if (!found) return [...settled, toolCallEntry]
      return settled.flatMap(item =>
        item.id === assistantMessageId ? [toolCallEntry, item] : [item]
      )
    })
    setMessageAnimationStates(prev => new Set([...prev, toolCallId]))
    syncMetadata(data)
  }

  const handleComplete = (data: { conversation_id?: string; remaining_limit?: number }) => {
    finishToolCalls()
    syncMetadata(data)
    cleanupConnection()
  }

  const handleStreamError = (data: { message?: string }, assistantMessageId: string) => {
    currentAssistantContentRef.current = `Sorry, I encountered an error: ${data.message || 'Server error'}`
    finishToolCalls()
    updateAssistantMessage(assistantMessageId, currentAssistantContentRef.current)
    cleanupConnection()
  }

  const handleRawChunk = (rawData: string, assistantMessageId: string) => {
    currentAssistantContentRef.current += rawData
    finishToolCalls()
    updateAssistantMessage(assistantMessageId, currentAssistantContentRef.current)
  }

  // --- Stream Response ---

  const streamResponse = (message: string, assistantMessageId: string) => {
    startSlowResponseTimeout(assistantMessageId)

    try {
      const params = new URLSearchParams({
        p: message,
        client: version,
        ...(conversationId && { conversation_id: conversationId }),
        ...(token && { token }),
      })

      const eventSource = new EventSource(`https://aac.gaijin.dev/ai/chat/stream?${params}`)
      activeEventSourceRef.current = eventSource

      eventSource.onmessage = event => {
        clearTimeoutRef(slowResponseTimeoutRef)
        try {
          const data = JSON.parse(event.data)
          switch (data.type) {
            case 'message':
              handleMessageChunk(data, assistantMessageId)
              break
            case 'tool_call':
              handleToolCall(data, assistantMessageId)
              break
            case 'complete':
              handleComplete(data)
              break
            case 'error':
              handleStreamError(data, assistantMessageId)
              break
          }
        } catch {
          console.error('Error parsing SSE data')
          handleRawChunk(event.data, assistantMessageId)
        }
      }

      eventSource.onerror = () => {
        clearTimeoutRef(slowResponseTimeoutRef)
        if (!currentAssistantContentRef.current) {
          currentAssistantContentRef.current =
            'Sorry, I encountered an error processing your request.'
        }
        finishToolCalls()
        updateAssistantMessage(assistantMessageId, currentAssistantContentRef.current)
        cleanupConnection()
      }
    } catch {
      clearTimeoutRef(slowResponseTimeoutRef)
      currentAssistantContentRef.current = 'Sorry, I encountered an error processing your request.'
      finishToolCalls()
      updateAssistantMessage(assistantMessageId, currentAssistantContentRef.current)
      cleanupConnection()
    }
  }

  // --- Public API ---

  const sendMessage = async (message: string) => {
    if (!message.trim() || isWaitingForResponse) return

    cleanupConnection()

    const userMessageId = generateMessageId()
    setChatHistory(prev => [
      ...prev,
      { id: userMessageId, type: 'message', role: 'user', content: message },
    ])
    setMessageAnimationStates(prev => new Set([...prev, userMessageId]))

    const assistantMessageId = generateMessageId()
    currentAssistantContentRef.current = ''
    setChatHistory(prev => [
      ...prev,
      { id: assistantMessageId, type: 'message', role: 'assistant', content: '' },
    ])
    setMessageAnimationStates(prev => new Set([...prev, assistantMessageId]))

    setIsWaitingForResponse(true)
    streamResponse(message, assistantMessageId)
  }

  return {
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
  }
}

export const useMarkdownSetup = () => {
  useEffect(() => {
    marked.setOptions({ gfm: true, breaks: true })

    DOMPurify.addHook('afterSanitizeAttributes', function (node) {
      if (node.tagName === 'A' && node.hasAttribute('href')) {
        const url = node.getAttribute('href')
        if (url) {
          node.setAttribute('href', 'javascript:void(0)')
          node.setAttribute('data-url', url)
          node.classList.add('wails-link')
        }
      }
    })

    return () => {
      DOMPurify.removeHook('afterSanitizeAttributes')
    }
  }, [])
}

export const useWailsLinkHandler = () => {
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
}

export const useAutoScroll = (
  chatHistory: ChatHistoryItem[],
  chatContainerRef: React.RefObject<HTMLDivElement | null>,
  messagesRef: React.RefObject<HTMLDivElement | null>
) => {
  useEffect(() => {
    const container = chatContainerRef.current
    if (chatHistory.length === 0 || !container) return

    const timer = setTimeout(() => {
      container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' })
    }, 50)

    return () => clearTimeout(timer)
  }, [chatHistory, chatContainerRef])

  useEffect(() => {
    const container = chatContainerRef.current
    const messages = messagesRef.current
    if (chatHistory.length === 0 || !container || !messages) return

    const scrollIfNearBottom = (behavior: ScrollBehavior) => {
      const distanceFromBottom =
        container.scrollHeight - container.scrollTop - container.clientHeight
      if (distanceFromBottom < 150) {
        container.scrollTo({ top: container.scrollHeight, behavior })
      }
    }

    const observer = new ResizeObserver(() => {
      scrollIfNearBottom('auto')
    })

    observer.observe(messages)
    return () => observer.disconnect()
  }, [chatHistory.length, chatContainerRef, messagesRef])
}

export const useInputFocus = (
  isInputDisabled: boolean,
  open: boolean,
  inputRef: React.RefObject<HTMLInputElement | null>
) => {
  useEffect(() => {
    if (!isInputDisabled && inputRef.current && open) {
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [isInputDisabled, open, inputRef])
}

export const useAnimationCleanup = (
  messageAnimationStates: Set<string>,
  setMessageAnimationStates: (states: Set<string>) => void
) => {
  useEffect(() => {
    if (messageAnimationStates.size > 0) {
      const timer = setTimeout(() => {
        setMessageAnimationStates(new Set())
      }, 1000)
      return () => clearTimeout(timer)
    }
  }, [messageAnimationStates, setMessageAnimationStates])
}
