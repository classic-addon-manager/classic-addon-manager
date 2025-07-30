<script lang="ts">
  import {Browser} from '@wailsio/runtime'
  import DOMPurify from 'dompurify'
  import {marked} from 'marked'
  import {tick} from 'svelte'

  import supportDaru from '@/assets/images/support_daru.webp'
  import supportDaruAlt from '@/assets/images/support_daru_alt_sm.webp'
  import {toast} from '@/utils'
  import {getVersion} from '$atoms/application.svelte'
  import {getToken, type User} from '$atoms/user.svelte'
  import * as Avatar from '$lib/components/ui/avatar/index'
  import {Button} from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog/index'

  let {open = $bindable(false), user} = $props<{ open?: boolean, user: User }>()

  // Consolidated state
  let chatMessage: string = $state('')
  let chatHistory: { role: 'user' | 'assistant', content: string, id: string }[] = $state([])
  let isWaitingForResponse: boolean = $state(false)
  let conversationId: string | null = $state(null)
  let remainingLimit: number = $state(0)

  // DOM references
  let chatContainer: HTMLDivElement
  let messageInput: HTMLInputElement

  // SSE management
  let activeEventSource: EventSource | null = $state(null)

  // Animation tracking - simplified
  let messageAnimationStates: Set<string> = $state(new Set())

  // Initialize markdown and DOMPurify once
  $effect(() => {
    marked.setOptions({gfm: true, breaks: true})

    DOMPurify.addHook('afterSanitizeAttributes', function (node) {
      if (node.tagName === 'A' && node.hasAttribute('href')) {
        const url = node.getAttribute('href')
        if (url) {
          node.setAttribute('href', 'javascript:void(0)')
          node.setAttribute('data-url', url)
          node.classList.add('wails-link')
          node.setAttribute('onclick', `event.preventDefault(); window.runtime.Browser.OpenURL('${url.replace(/'/g, '\\\'')}');`)
        }
      }
    })

    return () => DOMPurify.removeHook('afterSanitizeAttributes')
  })

  // Auto-scroll to bottom
  $effect(() => {
    if (chatHistory.length > 0) {
      tick().then(() => {
        setTimeout(() => {
          chatContainer?.scrollTo({
            top: chatContainer.scrollHeight,
            behavior: 'smooth'
          })
        }, 50)
      })
    }
  })

  // Focus input when not waiting
  $effect(() => {
    if (!isWaitingForResponse && messageInput && open) {
      setTimeout(() => messageInput?.focus(), 0)
    }
  })

  // Clean up animations after delay
  $effect(() => {
    if (messageAnimationStates.size > 0) {
      const timer = setTimeout(() => {
        messageAnimationStates.clear()
      }, 1000)
      return () => clearTimeout(timer)
    }
  })

  // Cleanup SSE on close/unmount
  $effect(() => {
    if (!open && activeEventSource) {
      cleanupConnection()
    }
    return () => cleanupConnection()
  })

  function cleanupConnection() {
    if (activeEventSource) {
      activeEventSource.close()
      activeEventSource = null
    }
    isWaitingForResponse = false
  }

  function parseMarkdown(content: string) {
    return DOMPurify.sanitize(
      marked.parse(content, {async: false}),
      {ADD_ATTR: ['data-url', 'onclick']}
    )
  }

  function generateMessageId() {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  }

  async function sendMessage() {
    if (!chatMessage.trim() || isWaitingForResponse) return

    cleanupConnection()

    // Add user message
    const userMessageId = generateMessageId()
    const userMessage = chatMessage
    chatMessage = ''

    chatHistory = [...chatHistory, {role: 'user', content: userMessage, id: userMessageId}]
    messageAnimationStates.add(userMessageId)

    // Add empty assistant message for streaming
    const assistantMessageId = generateMessageId()
    chatHistory = [...chatHistory, {role: 'assistant', content: '', id: assistantMessageId}]
    const assistantMessageIndex = chatHistory.length - 1
    messageAnimationStates.add(assistantMessageId)

    isWaitingForResponse = true

    // Timeout for slow response
    const timeoutId = setTimeout(() => {
      if (isWaitingForResponse && !chatHistory[assistantMessageIndex].content) {
        chatHistory[assistantMessageIndex].content = 'Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸'
      }
    }, 10000)

    try {
      const params = new URLSearchParams({
        p: userMessage,
        client: getVersion(),
        ...(conversationId && {conversation_id: conversationId}),
        ...(getToken() && {token: getToken()!})
      })

      const eventSource = new EventSource(`https://aac.gaijin.dev/ai/chat/stream?${params}`)
      activeEventSource = eventSource

      eventSource.onmessage = (event) => {
        clearTimeout(timeoutId)

        try {
          const data = JSON.parse(event.data)

          if (data.type === 'message') {
            chatHistory[assistantMessageIndex].content += (data.data || '')

            if (data.conversation_id) conversationId = data.conversation_id
            if (data.remaining_limit !== undefined) remainingLimit = data.remaining_limit
          } else if (data.type === 'complete') {
            if (data.conversation_id) conversationId = data.conversation_id
            if (data.remaining_limit !== undefined) remainingLimit = data.remaining_limit
            cleanupConnection()
          } else if (data.type === 'error') {
            chatHistory[assistantMessageIndex].content = `Sorry, I encountered an error: ${data.message || 'Server error'}`
            cleanupConnection()
          }
        } catch (parseError) {
          // Treat as raw content chunk
          chatHistory[assistantMessageIndex].content += event.data
        }
      }

      eventSource.onerror = () => {
        clearTimeout(timeoutId)
        if (!chatHistory[assistantMessageIndex].content) {
          chatHistory[assistantMessageIndex].content = 'Sorry, I encountered an error processing your request.'
        }
        cleanupConnection()
      }

    } catch (error) {
      clearTimeout(timeoutId)
      chatHistory[assistantMessageIndex].content = 'Sorry, I encountered an error processing your request.'
      cleanupConnection()
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text)
      toast.success('Copied to clipboard')
    } catch (err) {
      toast.error('Failed to copy text')
    }
  }
</script>

<!-- Keep the same template structure but simplify animations -->
<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm"/>
    <Dialog.Content
      class="fixed left-[50%] top-[50%] z-50 w-[calc(100%-2rem)] max-w-[70%] translate-x-[-50%] translate-y-[-50%] gap-0 border bg-background shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-xl overflow-hidden">
      <div class="flex max-h-[calc(100vh-4rem)] h-[650px] lg:h-[80vh] flex-col">
        <!-- Header -->
        <div class="border-b border-border/40 px-4 sm:px-6 py-4">
          <div class="flex items-center justify-between">
            <div>
              <Dialog.Title class="text-lg font-medium leading-none">Daru Informational Network
              </Dialog.Title>
              <Dialog.Description class="mt-2 text-sm text-muted-foreground">
                The Darus have information, if you have coin.
              </Dialog.Description>
            </div>
          </div>
        </div>

        <!-- Chat Area -->
        <div bind:this={chatContainer}
          class="flex-1 overflow-y-auto p-4 sm:p-6 scrollbar-thin scrollbar-thumb-secondary scrollbar-track-secondary/20">
          <div class="h-full">
            {#if chatHistory.length === 0}
              <div class="h-full flex flex-col items-center justify-center space-y-6 text-center px-4">
                <div class="space-y-6 max-h-full">
                  <img src={supportDaru} alt="Support Daru"
                    class="w-[35vh] h-[35vh] max-w-[280px] max-h-[280px] min-w-[160px] min-h-[160px] object-contain mx-auto drop-shadow-lg hover:scale-105 transition-transform duration-300"/>
                  <div>
                    <h3 class="text-xl font-semibold text-primary mb-2">Welcome to Daru's Help
                      Desk!</h3>
                    <p class="text-sm text-muted-foreground max-w-md mx-auto leading-relaxed">
                      I'm your friendly Daru assistant, ready to help with all your addon needs.
                      Feel free to ask me anything!
                    </p>
                  </div>
                </div>
              </div>
            {:else}
              <div class="space-y-6">
                {#each chatHistory as message, i (message.id)}
                  {#if message.role === 'user' || (message.role === 'assistant' && message.content.trim())}
                    <div class="flex items-start gap-3 px-4 {message.role === 'user' ? 'justify-end' : ''} 
                      {messageAnimationStates.has(message.id) ? 'message-fade-in' : ''}">
                      {#if message.role === 'assistant'}
                        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                          <img src={supportDaruAlt} alt="Daru Assistant"
                            class="w-full h-full object-cover"/>
                        </div>
                      {/if}

                      <div class="flex flex-col gap-2 {message.role === 'assistant' ? 'max-w-[calc(100%-3.5rem)] w-full' : 'max-w-[80%]'}">
                        <div class="relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message 
                          {message.role === 'assistant' ? 'assistant-message' : ''}
                          {message.role === 'user' ? 'bg-primary text-black' : 'bg-muted/50'}">
                          {#if message.role === 'assistant'}
                            {@html parseMarkdown(message.content)}
                            <button class="copy-button"
                              onclick={() => copyToClipboard(message.content)}>
                              <svg xmlns="http://www.w3.org/2000/svg" width="14"
                                height="14" viewBox="0 0 24 24" fill="none"
                                stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round">
                                <rect width="8" height="4" x="8" y="2" rx="1" ry="1"/>
                                <path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"/>
                                <path d="M16 4h2a2 2 0 0 1 2 2v4"/>
                                <path d="M21 14H11"/>
                                <path d="m15 10-4 4 4 4"/>
                              </svg>
                            </button>
                          {:else}
                            {message.content}
                          {/if}
                        </div>
                      </div>

                      {#if message.role === 'user'}
                        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-secondary">
                          <Avatar.Root class="h-full w-full">
                            <Avatar.Image
                              src={`https://cdn.discordapp.com/avatars/${user.discord_id}/${user.avatar}.png`}
                              alt={user.username} class="h-full w-full object-cover"/>
                            <Avatar.Fallback
                              class="text-xs">{user.username.substring(0, 2).toUpperCase()}</Avatar.Fallback>
                          </Avatar.Root>
                        </div>
                      {/if}
                    </div>
                  {/if}
                {/each}

                {#if isWaitingForResponse}
                  <div class="flex items-start gap-3 px-4">
                    <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                      <img src={supportDaruAlt} alt="Daru Assistant"
                        class="w-full h-full object-cover"/>
                    </div>
                    <div class="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50 border">
                      <div class="flex gap-1.5 items-center">
                        <span class="text-muted-foreground/70 text-xs mr-2">Daru is thinking</span>
                        <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"></span>
                        <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
                          style="animation-delay: 0.1s"></span>
                        <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
                          style="animation-delay: 0.2s"></span>
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        {#if remainingLimit > 0}
          <div class="flex justify-end px-4 pb-2">
            <div class="text-xs text-muted-foreground/60 flex items-center gap-1 px-2 py-0.5 rounded-full bg-muted/20">
              <span>{remainingLimit} messages remaining</span>
            </div>
          </div>
        {/if}

        <!-- Input Area -->
        <div class="border-t bg-background pt-3 sm:pt-4">
          <form class="flex items-center gap-2" onsubmit={(e) => { e.preventDefault(); sendMessage() }}>
            <div class="relative flex-1">
              <input
                bind:this={messageInput}
                type="text"
                bind:value={chatMessage}
                placeholder={isWaitingForResponse ? 'Daru is thinking...' : 'Ask your question, the Darus won\'t judge...'}
                disabled={isWaitingForResponse}
                autofocus
                class="w-full rounded-full border bg-muted/30 px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 pr-16 disabled:opacity-50"
              />

              <Button
                type="submit"
                size="icon"
                class="absolute right-1.5 top-1/2 -translate-y-1/2 transform rounded-full p-1.5"
                variant="ghost"
                disabled={!chatMessage.trim() || isWaitingForResponse}
              >
                {#if isWaitingForResponse}
                  <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round" class="text-primary">
                    <path d="m22 2-7 20-4-9-9-4Z"/>
                    <path d="M22 2 11 13"/>
                  </svg>
                {/if}
              </Button>
            </div>
          </form>
          <div class="flex items-center justify-center mt-2 min-h-[24px]">
            <p class="text-center text-xs text-muted-foreground/80">
              {isWaitingForResponse ? 'Daru is thinking...' : 'Darus are known for their wisdom, but sometimes even they make mistakes.'}
            </p>
          </div>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<svelte:window on:click={(e: MouseEvent) => {
  if (e.target instanceof HTMLElement) {
    const target = e.target
    if (!target.classList.contains('wails-link')) {
      return
    }
    const url = target.getAttribute('data-url')
    if (url) Browser.OpenURL(url)
  }
}}/>

<style>
    :global(.chat-message), :global(.chat-message *) {
        user-select: text !important;
    }

    :global(.chat-message.assistant-message) {
        @apply prose prose-invert max-w-none;
        max-width: 100%;
        overflow-wrap: break-word;
    }

    /* Markdown styling for assistant messages */
    :global(.chat-message.assistant-message h1),
    :global(.chat-message.assistant-message h2),
    :global(.chat-message.assistant-message h3),
    :global(.chat-message.assistant-message h4),
    :global(.chat-message.assistant-message h5),
    :global(.chat-message.assistant-message h6) {
        @apply font-semibold mt-4 mb-2 text-foreground;
    }

    :global(.chat-message.assistant-message h1) {
        @apply text-xl;
    }

    :global(.chat-message.assistant-message h2) {
        @apply text-lg;
    }

    :global(.chat-message.assistant-message h3) {
        @apply text-base;
    }

    :global(.chat-message.assistant-message p) {
        @apply mb-3 last:mb-0 leading-relaxed text-foreground;
    }

    :global(.chat-message.assistant-message ul),
    :global(.chat-message.assistant-message ol) {
        @apply mb-3 pl-4 space-y-1;
    }

    :global(.chat-message.assistant-message li) {
        @apply text-foreground;
    }

    :global(.chat-message.assistant-message ul li) {
        @apply list-disc;
    }

    :global(.chat-message.assistant-message ol li) {
        @apply list-decimal;
    }

    :global(.chat-message.assistant-message blockquote) {
        @apply border-l-4 border-primary/30 pl-4 py-2 my-3 bg-muted/30 rounded-r text-muted-foreground italic;
    }

    :global(.chat-message.assistant-message a),
    :global(.chat-message.assistant-message .wails-link) {
        @apply text-blue-400 hover:text-blue-500 hover:underline transition-all cursor-pointer;
    }

    :global(.chat-message.assistant-message code) {
        @apply bg-gray-700 py-0.5 px-1 rounded-sm font-mono text-sm;
    }

    :global(.chat-message.assistant-message code::before),
    :global(.chat-message.assistant-message code::after) {
        content: none !important;
    }

    :global(.chat-message.assistant-message pre) {
        @apply bg-neutral-900 p-4 my-4 overflow-x-auto rounded-md font-mono text-muted-foreground border;
    }

    :global(.chat-message.assistant-message pre code) {
        @apply bg-transparent p-0 text-sm;
    }

    :global(.chat-message.assistant-message strong) {
        @apply font-semibold text-foreground;
    }

    :global(.chat-message.assistant-message em) {
        @apply italic text-muted-foreground;
    }

    :global(.chat-message.assistant-message hr) {
        @apply border-t border-border my-4;
    }

    :global(.chat-message.assistant-message table) {
        @apply w-full border-collapse border border-border my-3;
    }

    :global(.chat-message.assistant-message th),
    :global(.chat-message.assistant-message td) {
        @apply border border-border px-3 py-2 text-left;
    }

    :global(.chat-message.assistant-message th) {
        @apply bg-muted font-semibold;
    }

    .copy-button {
        @apply absolute top-1 right-1 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-secondary/50 transition-all duration-150;
        opacity: 0;
        visibility: hidden;
    }

    :global(.chat-message:hover .copy-button) {
        opacity: 1;
        visibility: visible;
    }

    :global(.message-fade-in) {
        animation: fadeInMessage 0.6s cubic-bezier(0.16, 1, 0.3, 1);
    }

    @keyframes fadeInMessage {
        from {
            opacity: 0;
            transform: translateY(8px) scale(0.98);
        }
        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }
</style>