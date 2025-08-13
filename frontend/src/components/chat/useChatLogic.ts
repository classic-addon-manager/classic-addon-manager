import { Browser } from '@wailsio/runtime'
import DOMPurify from 'dompurify'
import { useAtomValue } from 'jotai'
import { marked } from 'marked'
import { useCallback, useEffect, useRef, useState } from 'react'

import { versionAtom } from '@/atoms/applicationAtoms'
import { toast } from '@/components/ui/toast'
import { useUserStore } from '@/stores/userStore'

import type { ChatMessageType } from './types'

export const useChatLogic = () => {
  const { token } = useUserStore()
  const version = useAtomValue(versionAtom)

  // State
  const [chatHistory, setChatHistory] = useState<ChatMessageType[]>([])
  const [isWaitingForResponse, setIsWaitingForResponse] = useState(false)
  const [conversationId, setConversationId] = useState<string | null>(null)
  const [remainingLimit, setRemainingLimit] = useState(0)
  const [messageAnimationStates, setMessageAnimationStates] = useState<Set<string>>(new Set())

  // Refs
  const activeEventSourceRef = useRef<EventSource | null>(null)
  const currentAssistantContentRef = useRef<string>('')

  const generateMessageId = () => {
    return `msg_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
  }

  const cleanupConnection = useCallback(() => {
    if (activeEventSourceRef.current) {
      activeEventSourceRef.current.close()
      activeEventSourceRef.current = null
    }
    setIsWaitingForResponse(false)
  }, [])

  const parseMarkdown = (content: string) => {
    return DOMPurify.sanitize(marked.parse(content, { async: false }) as string, {
      ADD_ATTR: ['data-url', 'onclick'],
    })
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

  const sendMessage = async (message: string) => {
    if (!message.trim() || isWaitingForResponse) return

    cleanupConnection()

    // Add user message
    const userMessageId = generateMessageId()
    setChatHistory(prev => [...prev, { role: 'user', content: message, id: userMessageId }])
    setMessageAnimationStates(prev => new Set([...prev, userMessageId]))

    // Add empty assistant message for streaming
    const assistantMessageId = generateMessageId()
    currentAssistantContentRef.current = '' // Reset content ref
    setChatHistory(prev => [...prev, { role: 'assistant', content: '', id: assistantMessageId }])
    setMessageAnimationStates(prev => new Set([...prev, assistantMessageId]))

    setIsWaitingForResponse(true)

    // Timeout for slow response
    const timeoutId = setTimeout(() => {
      setChatHistory(prev => {
        const newHistory = [...prev]
        const assistantMessageIndex = newHistory.findIndex(msg => msg.id === assistantMessageId)
        if (assistantMessageIndex !== -1 && !newHistory[assistantMessageIndex].content) {
          newHistory[assistantMessageIndex].content =
            'Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸'
        }
        return newHistory
      })
    }, 10000)

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
          console.error('Error parsing SSE data:', parseError)
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
    } catch {
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

  return {
    chatHistory,
    isWaitingForResponse,
    remainingLimit,
    messageAnimationStates,
    sendMessage,
    parseMarkdown,
    copyToClipboard,
    cleanupConnection,
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
          node.setAttribute(
            'onclick',
            `event.preventDefault(); window.runtime.Browser.OpenURL('${url.replace(/'/g, "\\'")}');`
          )
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
  chatHistory: ChatMessageType[],
  chatContainerRef: React.RefObject<HTMLDivElement | null>
) => {
  useEffect(() => {
    if (chatHistory.length > 0 && chatContainerRef.current) {
      setTimeout(() => {
        chatContainerRef.current?.scrollTo({
          top: chatContainerRef.current.scrollHeight,
          behavior: 'smooth',
        })
      }, 50)
    }
  }, [chatHistory, chatContainerRef])
}

export const useInputFocus = (
  isWaitingForResponse: boolean,
  open: boolean,
  inputRef: React.RefObject<HTMLInputElement | null>
) => {
  useEffect(() => {
    if (!isWaitingForResponse && inputRef.current && open) {
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [isWaitingForResponse, open, inputRef])
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
