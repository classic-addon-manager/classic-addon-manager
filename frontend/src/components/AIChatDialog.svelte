<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import * as Avatar from "$lib/components/ui/avatar/index";
    import {Button} from "$lib/components/ui/button";
    import {tick} from "svelte";
    import {apiClient} from "../api";
    import {toast} from "../utils";
    import supportDaru from "../assets/images/support_daru.webp";
    import supportDaruAlt from "../assets/images/support_daru_alt_sm.webp";
    import DOMPurify from "dompurify";
    import {marked} from "marked";
    import type {User} from "$stores/UserStore.svelte";
    import {getToken} from "$stores/UserStore.svelte";
    import {getVersion} from "$stores/ApplicationStore.svelte";
    import {Browser} from "@wailsio/runtime";

    let { open = $bindable(false), user } = $props<{open?: boolean, user: User}>();

    let chatMessage: string = $state('');
    let chatHistory: { role: 'user' | 'assistant', content: string, id: string }[] = $state([]);
    let isWaitingForResponse: boolean = $state(false);
    let chatContainer: HTMLDivElement;
    let messageInput: HTMLInputElement;
    let lastMessageId: string = $state('');
    
    // Add new state variables for conversation tracking
    let conversationId: string | null = $state(null);
    let remainingLimit: number = $state(0);
    
    // SSE connection management
    let activeEventSource: EventSource | null = $state(null);
    let cleanupFunction: (() => void) | null = null;

    // Animation state tracking
    let isStreaming: boolean = $state(false);
    let streamingMessageIndex: number = $state(-1);
    let messageAnimationStates: Map<string, number> = $state(new Map());
    
    // Trigger for streaming scroll updates
    let streamingContentUpdateTrigger: number = $state(0);

    // Configure marked options
    $effect(() => {
        marked.setOptions({
            gfm: true,
            breaks: true,
        });
    });

    // Configure DOMPurify hooks to handle links
    $effect(() => {
        // Configure DOMPurify to handle links with custom behavior
        DOMPurify.addHook('afterSanitizeAttributes', function(node) {
            if (node.tagName === 'PRE' || node.tagName === 'CODE') {
                node.className = node.className || '';
            }
            
            // Handle links with Wails
            if (node.tagName === 'A' && node.hasAttribute('href')) {
                const url = node.getAttribute('href');
                if (url) {
                    // Keep the href for accessibility but make it a javascript: protocol
                    node.setAttribute('href', `javascript:void(0)`);
                    // Store the original URL for the click handler
                    node.setAttribute('data-url', url);
                    // Mark as a Wails link for styling
                    node.classList.add('wails-link');
                    // Add onclick handler that uses Wails Browser
                    node.setAttribute('onclick', `event.preventDefault(); window.runtime.Browser.OpenURL('${url.replace(/'/g, "\\'")}');`);
                }
            }
        });

        // Clean up hook when component is destroyed
        return () => {
            DOMPurify.removeHook('afterSanitizeAttributes');
        };
    });

    $effect(() => {
        if (chatHistory.length > 0 || streamingContentUpdateTrigger > 0) {
            // Use tick() to wait for the next DOM update cycle
            tick().then(() => {
                setTimeout(() => {
                    const lastMessage = document.getElementById(lastMessageId);
                    if (lastMessage) {
                        lastMessage.scrollIntoView({ behavior: 'smooth' });
                    } else {
                        chatContainer?.scrollTo({
                            top: chatContainer.scrollHeight,
                            behavior: 'smooth'
                        });
                    }
                }, 50);
            });
        }
    });

    $effect(() => {
        if (!isWaitingForResponse && messageInput && open) {
            setTimeout(() => messageInput?.focus(), 0);
        }
    });

    // Clean up animation states for messages after they appear
    $effect(() => {
        if (messageAnimationStates.size > 0) {
            const now = Date.now();
            const expiredMessageIds: string[] = [];
            
            for (const [messageId, timestamp] of messageAnimationStates) {
                if (now - timestamp > 800) { // 800ms after animation start
                    expiredMessageIds.push(messageId);
                }
            }
            
            if (expiredMessageIds.length > 0) {
                const timer = setTimeout(() => {
                    for (const messageId of expiredMessageIds) {
                        messageAnimationStates.delete(messageId);
                    }
                    messageAnimationStates = new Map(messageAnimationStates); // Trigger reactivity
                }, 50);
                
                return () => clearTimeout(timer);
            }
        }
    });

    // Cleanup SSE connection when dialog is closed or component unmounts
    $effect(() => {
        if (!open && cleanupFunction) {
            cleanupFunction();
            cleanupFunction = null;
        }
        
        // Cleanup on component destroy
        return () => {
            if (cleanupFunction) {
                cleanupFunction();
                cleanupFunction = null;
            }
        };
    });

    // Parse and sanitize markdown content
    function parseMarkdown(content: string) {
        return DOMPurify.sanitize(
            marked.parse(content, {async: false}), 
            {ADD_ATTR: ['data-url', 'onclick']}
        );
    }

    async function sendMessage() {
        if (!chatMessage.trim() || isWaitingForResponse) return;

        // Clean up any existing connection
        if (cleanupFunction) {
            cleanupFunction();
            cleanupFunction = null;
        }

        // Add user message to chat history
        const userMessageId = `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        chatHistory = [...chatHistory, {role: 'user', content: chatMessage, id: userMessageId}];
        lastMessageId = `message-${chatHistory.length - 1}`;
        
        // Mark new user message for animation
        messageAnimationStates.set(userMessageId, Date.now());

        // Clear input field
        const userMessage = chatMessage;
        chatMessage = '';
        
        // Set loading state
        isWaitingForResponse = true;
        isStreaming = false;

        // Add empty assistant message that will be populated by streaming
        const assistantMessageId = `msg_${Date.now() + 1}_${Math.random().toString(36).substr(2, 9)}`;
        chatHistory = [...chatHistory, {role: 'assistant', content: '', id: assistantMessageId}];
        const assistantMessageIndex = chatHistory.length - 1;
        streamingMessageIndex = assistantMessageIndex;
        lastMessageId = `message-${assistantMessageIndex}`;
        
        // Mark new assistant message for animation
        messageAnimationStates.set(assistantMessageId, Date.now());

        // Start timeout for slow response message
        let timeoutId: number | undefined = undefined;
        let hasReceivedData = false;
        
        timeoutId = setTimeout(() => {
            if (isWaitingForResponse && !hasReceivedData) {
                chatHistory[assistantMessageIndex].content = "Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸";
                // Trigger scroll update
                streamingContentUpdateTrigger++;
            }
        }, 10000); // 10 seconds

        let eventSource: EventSource | null = null;

        try {
            // Create SSE connection with query parameters
            const params = new URLSearchParams({
                p: userMessage,
                ...(conversationId ? { conversation_id: conversationId } : {})
            });
            
            // Add authentication headers as query params for SSE (since EventSource doesn't support custom headers)
            params.append('client', getVersion());
            const token = getToken();
            if (token) {
                params.append('token', token);
            }
            
            // Use the same API base URL and endpoint from api.ts
            const sseUrl = `https://aac.gaijin.dev/ai/chat/stream?${params.toString()}`;
            
            eventSource = new EventSource(sseUrl);
            activeEventSource = eventSource;

            eventSource.onmessage = (event) => {
                hasReceivedData = true;
                isStreaming = true;
                
                if (timeoutId) {
                    clearTimeout(timeoutId);
                    timeoutId = undefined;
                }
                
                try {
                    const data = JSON.parse(event.data);
                    
                    if (data.type === 'message') {
                        // Append content chunk to the assistant message
                        chatHistory[assistantMessageIndex].content += (data.data || '');
                        // Trigger scroll update for streaming content
                        streamingContentUpdateTrigger++;
                        
                        // Update conversation metadata if provided
                        if (data.conversation_id) {
                            conversationId = data.conversation_id;
                        }
                        if (data.remaining_limit !== undefined) {
                            remainingLimit = data.remaining_limit;
                        }
                    } else if (data.type === 'complete') {
                        // Update final conversation metadata
                        if (data.conversation_id) {
                            conversationId = data.conversation_id;
                        }
                        if (data.remaining_limit !== undefined) {
                            remainingLimit = data.remaining_limit;
                        }
                        
                        // Streaming is complete
                        isStreaming = false;
                        streamingMessageIndex = -1;
                        eventSource?.close();
                        activeEventSource = null;
                        isWaitingForResponse = false;
                        cleanupFunction = null;
                    } else if (data.type === 'error') {
                        const errorMessage = data.message || 'Server error during streaming';
                        
                        chatHistory[assistantMessageIndex].content = `Sorry, I encountered an error: ${errorMessage}`;
                        // Trigger scroll update
                        streamingContentUpdateTrigger++;
                        
                        isStreaming = false;
                        streamingMessageIndex = -1;
                        eventSource?.close();
                        activeEventSource = null;
                        if (timeoutId) {
                            clearTimeout(timeoutId);
                            timeoutId = undefined;
                        }
                        isWaitingForResponse = false;
                        cleanupFunction = null;
                    }
                } catch (parseError) {
                    console.error('Error parsing SSE data:', parseError);
                    // If we can't parse, treat as raw content chunk
                    chatHistory[assistantMessageIndex].content += event.data;
                    // Trigger scroll update for streaming content
                    streamingContentUpdateTrigger++;
                }
            };

            eventSource.onerror = (error) => {
                console.error('SSE error:', error);
                eventSource?.close();
                activeEventSource = null;
                
                if (!hasReceivedData) {
                    // If no data was received, show error message
                    chatHistory[assistantMessageIndex].content = 'Sorry, I encountered an error processing your request.';
                    // Trigger scroll update
                    streamingContentUpdateTrigger++;
                }
                
                isStreaming = false;
                streamingMessageIndex = -1;
                if (timeoutId) {
                    clearTimeout(timeoutId);
                    timeoutId = undefined;
                }
                isWaitingForResponse = false;
                cleanupFunction = null;
            };

            eventSource.onopen = () => {
                console.log('SSE connection opened');
            };

            // Set up cleanup function
            cleanupFunction = () => {
                eventSource?.close();
                activeEventSource = null;
                if (timeoutId) {
                    clearTimeout(timeoutId);
                    timeoutId = undefined;
                }
                isStreaming = false;
                streamingMessageIndex = -1;
                isWaitingForResponse = false;
            };

        } catch (error) {
            console.error('Chat error:', error);
            
            // If there's an error setting up SSE, add an error message
            chatHistory[assistantMessageIndex].content = 'Sorry, I encountered an error processing your request.';
            // Trigger scroll update
            streamingContentUpdateTrigger++;
            
            isStreaming = false;
            streamingMessageIndex = -1;
            if (timeoutId) {
                clearTimeout(timeoutId);
                timeoutId = undefined;
            }
            isWaitingForResponse = false;
            cleanupFunction = null;
        }
    }

    async function copyToClipboard(text: string) {
        try {
            await navigator.clipboard.writeText(text);
            toast.success('Copied to clipboard');
        } catch (err) {
            toast.error('Failed to copy text');
            console.error('Failed to copy: ', err);
        }
    }
</script>

<style>
    /* Markdown styling for chat messages */
    :global(.chat-message) {
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message) {
        @apply prose prose-invert max-w-none;
        max-width: 100%;
        overflow-wrap: break-word;
        word-wrap: break-word;
        word-break: break-word;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message ol),
    :global(.chat-message.assistant-message ul) {
        margin-top: 0;
        margin-bottom: 0;
    }

    :global(.chat-message.assistant-message li) {
        margin-top: 0;
        margin-bottom: 0;
    }

    :global(.chat-message.assistant-message li > p) {
        margin: 0;
        display: inline;
    }

    :global(.chat-message.assistant-message li > ul),
    :global(.chat-message.assistant-message li > ol) {
        margin-top: 0.5rem;
    }

    :global(.chat-message.assistant-message a) {
        @apply text-blue-400 hover:text-blue-500 hover:underline transition-all cursor-pointer;
    }
    
    :global(.chat-message.assistant-message .wails-link) {
        @apply text-blue-400 hover:text-blue-500 hover:underline transition-all cursor-pointer;
    }

    :global(.chat-message.assistant-message code) {
        @apply bg-gray-700 py-0.5 px-1 rounded-sm font-mono;
        word-break: break-all;
    }

    /* Override Tailwind Prose backticks for inline code */
    :global(.chat-message.assistant-message code::before),
    :global(.chat-message.assistant-message code::after) {
        content: none !important;
    }

    :global(.chat-message.assistant-message pre) {
        @apply bg-neutral-900 p-4 my-4 overflow-x-auto rounded-md font-mono text-muted-foreground;
        max-width: 100%;
        width: 100%;
    }

    :global(.chat-message.assistant-message pre code) {
        @apply bg-transparent p-0 rounded-none;
        white-space: pre-wrap;
        word-break: break-word;
    }

    /* Language tag for code blocks */
    :global(.chat-message.assistant-message pre::before) {
        @apply text-xs opacity-60 block mb-2;
        content: attr(data-language);
        text-transform: uppercase;
    }

    :global(.chat-message.assistant-message p) {
        @apply my-2;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message ul) {
        @apply list-disc list-inside my-2;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message ol) {
        @apply list-decimal list-inside my-2;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message li) {
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message code) {
        @apply bg-gray-700 py-0.5 px-1 rounded-sm font-mono;
        word-break: break-all;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    :global(.chat-message.assistant-message pre code) {
        @apply bg-transparent p-0 rounded-none;
        white-space: pre-wrap;
        word-break: break-word;
        user-select: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
    }

    .copy-button {
        @apply absolute top-1 right-1 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-secondary/50 transition-all duration-150;
        user-select: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
        opacity: 0;
        visibility: hidden;
        transform: scale(0.9);
    }

    /* Show copy button on message hover */
    :global(.chat-message:hover .copy-button) {
        opacity: 1;
        visibility: visible;
        transform: scale(1);
    }

    /* Animation for bounce dots */
    :global(.animate-bounce-dot-1) {
        animation: bounce-dot 1.8s ease-in-out infinite both;
        animation-delay: -0.4s;
    }

    :global(.animate-bounce-dot-2) {
        animation: bounce-dot 1.8s ease-in-out infinite both;
        animation-delay: -0.2s;
    }

    :global(.animate-bounce-dot-3) {
        animation: bounce-dot 1.8s ease-in-out infinite both;
    }

    @keyframes bounce-dot {
        0%, 70%, 100% {
            transform: scale(0.6);
            opacity: 0.4;
        }
        35% {
            transform: scale(1);
            opacity: 1;
        }
    }

    /* Message fade-in animation */
    :global(.message-fade-in) {
        animation: fadeInMessage 0.6s cubic-bezier(0.16, 1, 0.3, 1);
    }

    :global(.streaming-content) {
        transition: opacity 0.15s ease-out;
    }

    /* Smooth transitions for animated elements */
    :global(.message-fade-in) {
        will-change: transform, opacity;
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

    /* Enhanced waiting state */
    :global(.waiting-message) {
        background: linear-gradient(90deg, 
            rgba(156, 163, 175, 0.08) 0%, 
            rgba(156, 163, 175, 0.15) 50%, 
            rgba(156, 163, 175, 0.08) 100%);
        background-size: 200% 100%;
        animation: shimmer 3s ease-in-out infinite;
    }

    @keyframes shimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    /* Thinking indicator animations */
    .thinking-indicator-fade-in {
        animation: thinkingFadeIn 0.5s ease-out forwards;
    }

    @keyframes thinkingFadeIn {
        from {
            opacity: 0;
            transform: translateY(-8px) scale(0.95);
        }
        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }

    .thinking-message {
        background: linear-gradient(90deg, 
            rgba(59, 130, 246, 0.05) 0%, 
            rgba(59, 130, 246, 0.1) 50%, 
            rgba(59, 130, 246, 0.05) 100%);
        background-size: 200% 100%;
        animation: thinkingShimmer 2.5s ease-in-out infinite;
        border-color: rgba(59, 130, 246, 0.2);
    }

    @keyframes thinkingShimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    .avatar-thinking {
        animation: thinkingPulse 2s ease-in-out infinite;
    }

    @keyframes thinkingPulse {
        0%, 100% {
            transform: scale(1);
            box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.3);
        }
        50% {
            transform: scale(1.03);
            box-shadow: 0 0 0 6px rgba(59, 130, 246, 0);
        }
    }

    /* Input area connecting state */
    .input-connecting {
        background: linear-gradient(90deg, 
            rgba(59, 130, 246, 0.03) 0%, 
            rgba(59, 130, 246, 0.08) 50%, 
            rgba(59, 130, 246, 0.03) 100%);
        background-size: 200% 100%;
        animation: inputConnectShimmer 2s ease-in-out infinite;
        border-color: rgba(59, 130, 246, 0.2);
    }

    @keyframes inputConnectShimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    .input-connecting::placeholder {
        animation: placeholderPulse 2s ease-in-out infinite;
    }

    @keyframes placeholderPulse {
        0%, 100% {
            opacity: 0.6;
        }
        50% {
            opacity: 0.3;
        }
    }

    /* Network indicator dots */
    .network-indicator {
        position: relative;
        display: flex;
        align-items: center;
        gap: 2px;
    }

    .network-dot {
        width: 4px;
        height: 4px;
        border-radius: 50%;
        background-color: rgba(59, 130, 246, 0.6);
        animation: networkPulse 1.5s ease-in-out infinite;
    }

    .network-dot:nth-child(1) {
        animation-delay: 0s;
    }

    .network-dot:nth-child(2) {
        animation-delay: 0.3s;
    }

    .network-dot:nth-child(3) {
        animation-delay: 0.6s;
    }

    @keyframes networkPulse {
        0%, 60%, 100% {
            opacity: 0.3;
            transform: scale(0.8);
        }
        30% {
            opacity: 1;
            transform: scale(1.2);
        }
    }

    /* Connection waves effect */
    .connection-waves {
        position: absolute;
        right: 12px;
        top: 50%;
        transform: translateY(-50%);
        display: flex;
        align-items: center;
        gap: 1px;
    }

    .wave-bar {
        width: 2px;
        background: linear-gradient(to top, rgba(59, 130, 246, 0.4), rgba(59, 130, 246, 0.8));
        border-radius: 1px;
        animation: waveHeight 1.2s ease-in-out infinite;
    }

    .wave-bar:nth-child(1) {
        height: 8px;
        animation-delay: 0s;
    }

    .wave-bar:nth-child(2) {
        height: 12px;
        animation-delay: 0.1s;
    }

    .wave-bar:nth-child(3) {
        height: 16px;
        animation-delay: 0.2s;
    }

    .wave-bar:nth-child(4) {
        height: 12px;
        animation-delay: 0.3s;
    }

    .wave-bar:nth-child(5) {
        height: 8px;
        animation-delay: 0.4s;
    }

    @keyframes waveHeight {
        0%, 100% {
            transform: scaleY(0.3);
            opacity: 0.4;
        }
        50% {
            transform: scaleY(1);
            opacity: 1;
        }
    }
</style>

<Dialog.Root bind:open>
    <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
        <Dialog.Content
                class="fixed left-[50%] top-[50%] z-50 w-[calc(100%-2rem)] max-w-[70%] translate-x-[-50%] translate-y-[-50%] gap-0 border bg-background shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-xl overflow-hidden">
            <div class="flex max-h-[calc(100vh-4rem)] h-[650px] lg:h-[80vh] flex-col">
                <!-- Header -->
                <div class="border-b border-border/40 px-4 sm:px-6 py-4">
                    <div class="flex items-center justify-between">
                        <div>
                            <Dialog.Title class="text-lg font-medium leading-none">Daru Informational Network</Dialog.Title>
                            <Dialog.Description class="mt-2 text-sm text-muted-foreground">
                              The Darus have information, if you have coin.     
                            </Dialog.Description>
                        </div>
                    </div>
                </div>

                <!-- Chat Area -->
                <div 
                    bind:this={chatContainer}
                    class="flex-1 overflow-y-auto p-4 sm:p-6 scrollbar-thin scrollbar-thumb-secondary scrollbar-track-secondary/20">
                    <div class="h-full">
                        {#if chatHistory.length === 0}
                            <div class="h-full flex flex-col items-center justify-center space-y-6 text-center px-4">
                                <div class="space-y-6 max-h-full">
                                    <img 
                                        src={supportDaru} 
                                        alt="Support Daru" 
                                        class="w-[35vh] h-[35vh] max-w-[280px] max-h-[280px] min-w-[160px] min-h-[160px] object-contain mx-auto drop-shadow-lg hover:scale-105 transition-transform duration-300" 
                                    />
                                    <div>
                                        <h3 class="text-xl font-semibold text-primary mb-2">Welcome to Daru's Help Desk!</h3>
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
                                    <!-- Only show assistant messages that have content, or all user messages -->
                                    {#if message.role === 'user' || (message.role === 'assistant' && message.content.trim())}
                                        <!-- Thinking indicator above streaming message -->
                                        {#if message.role === 'assistant' && i === streamingMessageIndex && isStreaming && !message.content.trim()}
                                            <div class="flex items-start gap-3 px-4 thinking-indicator-fade-in">
                                                <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden avatar-thinking">
                                                    <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                                </div>
                                                <div class="flex flex-col gap-2 max-w-[calc(100%-3.5rem)] w-full">
                                                    <div class="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50 thinking-message border">
                                                        <div class="flex gap-1.5 items-center min-w-[2rem] min-h-[1.25rem]">
                                                            <span class="text-muted-foreground/70 text-xs mr-2">Daru is thinking</span>
                                                            <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-1"></span>
                                                            <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-2"></span>
                                                            <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-3"></span>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        {/if}

                                        <div id={`message-${i}`} 
                                             class="flex items-start gap-3 px-4 {message.role === 'user' ? 'justify-end' : ''} 
                                                    {messageAnimationStates.has(message.id) ? 'message-fade-in' : ''}"
                                        >
                                            {#if message.role === 'assistant'}
                                                <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                                                    <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                                </div>
                                            {/if}
                                            <div class="flex flex-col gap-2 {message.role === 'assistant' ? 'max-w-[calc(100%-3.5rem)] w-full' : 'max-w-[80%]'}">
                                                <div class="relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message 
                                                            {message.role === 'assistant' ? 'assistant-message' : ''}
                                                            {message.role === 'user' ? 'bg-primary text-black' : 'bg-muted/50'}"
                                                     style="{message.role === 'assistant' ? 'max-width: calc(100% - 1rem); width: 100%;' : 'max-width: 100%;'}"
                                                >
                                                    {#if message.role === 'assistant'}
                                                        <div class="relative streaming-content">
                                                            {@html parseMarkdown(message.content)}
                                                        </div>
                                                        <button class="copy-button" onclick={() => copyToClipboard(message.content)}>
                                                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-clipboard-copy"><rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"/><path d="M16 4h2a2 2 0 0 1 2 2v4"/><path d="M21 14H11"/><path d="m15 10-4 4 4 4"/></svg>
                                                            <span class="sr-only">Copy message</span>
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
                                                            alt={user.username}
                                                            class="h-full w-full object-cover"
                                                        />
                                                        <Avatar.Fallback class="text-xs">
                                                            {user.username.substring(0, 2).toUpperCase()}
                                                        </Avatar.Fallback>
                                                    </Avatar.Root>
                                                </div>
                                            {/if}
                                        </div>
                                    {/if}
                                {/each}
                                
                                {#if isWaitingForResponse && !isStreaming}
                                    <div class="flex items-start gap-3 px-4 message-fade-in">
                                        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                                            <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                        </div>
                                        <div class="flex flex-col gap-2">
                                            <div class="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50 waiting-message border">
                                                <div class="flex gap-1.5 items-center min-w-[2rem] min-h-[1.25rem]">
                                                    <span class="text-muted-foreground/70 text-xs mr-2">Connecting to Daru network</span>
                                                    <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-1"></span>
                                                    <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-2"></span>
                                                    <span class="w-1.5 h-1.5 bg-primary rounded-full animate-bounce-dot-3"></span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                {/if}
                                
                                <div class="h-1"></div>
                            </div>
                        {/if}
                    </div>
                </div>

                {#if remainingLimit > 0}
                <div class="flex justify-end px-4 pb-2">
                    <div class="text-xs text-muted-foreground/60 flex items-center gap-1 px-2 py-0.5 rounded-full bg-muted/20">
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="opacity-70">
                            <path d="M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48 2.83 2.83M2 12h4m12 0h4M4.93 19.07l2.83-2.83m8.48-8.48 2.83-2.83"/>
                        </svg>
                        <span>{remainingLimit} messages remaining</span>
                    </div>
                </div>
                {/if}

                <!-- Input Area -->
                <div class="border-t bg-background pt-3 sm:pt-4">
                    <form
                        class="flex items-center gap-2"
                        onsubmit={(e) => { e.preventDefault(); sendMessage(); }}
                    >
                        <div class="relative flex-1">
                            <input
                                bind:this={messageInput}
                                type="text"
                                bind:value={chatMessage}
                                placeholder={isWaitingForResponse ? "Daru is thinking..." : "Ask your question, the Darus won't judge..."}
                                disabled={isWaitingForResponse}
                                autofocus
                                class="w-full rounded-full border bg-muted/30 px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 pr-16 disabled:opacity-50 transition-all duration-300 {isWaitingForResponse ? 'input-connecting' : ''}"
                            />
                            
                            <!-- Connection indicator when waiting for response -->
                            {#if isWaitingForResponse}
                                <div class="connection-waves">
                                    <div class="wave-bar"></div>
                                    <div class="wave-bar"></div>
                                    <div class="wave-bar"></div>
                                    <div class="wave-bar"></div>
                                    <div class="wave-bar"></div>
                                </div>
                            {/if}
                            
                            <Button
                                type="submit"
                                size="icon"
                                class="absolute right-1.5 top-1/2 -translate-y-1/2 transform rounded-full p-1.5 hover:bg-primary/10 transition-colors duration-200 disabled:opacity-50"
                                variant="ghost"
                                disabled={!chatMessage.trim() || isWaitingForResponse}
                            >
                                {#if isWaitingForResponse}
                                    <div class="network-indicator">
                                        <div class="network-dot"></div>
                                        <div class="network-dot"></div>
                                        <div class="network-dot"></div>
                                    </div>
                                {:else}
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" 
                                         stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" 
                                         class="text-primary">
                                        <path d="m22 2-7 20-4-9-9-4Z"/>
                                        <path d="M22 2 11 13"/>
                                    </svg>
                                {/if}
                                <span class="sr-only">Send Message</span>
                            </Button>
                        </div>
                    </form>
                    <div class="flex items-center justify-center mt-2 min-h-[24px]">
                        {#if isWaitingForResponse}
                            <div class="flex items-center gap-2 text-xs text-muted-foreground/60 bg-muted/20 px-3 py-1.5 rounded-full">
                                <div class="network-indicator">
                                    <div class="network-dot"></div>
                                    <div class="network-dot"></div>
                                    <div class="network-dot"></div>
                                </div>
                                <span>Daru is thinking...</span>
                            </div>
                        {:else}
                            <p class="text-center text-xs text-muted-foreground/80">
                                Darus are known for their wisdom, but sometimes even they make mistakes.
                            </p>
                        {/if}
                    </div>
                </div>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>

<svelte:window on:click={(e: MouseEvent) => {
    // Handle link clicks globally
    const target = e.target as HTMLElement;
    if (target && target.classList && target.classList.contains('wails-link')) {
        e.preventDefault();
        const url = target.getAttribute('data-url');
        if (url) {
            Browser.OpenURL(url);
        }
    }
}}/>