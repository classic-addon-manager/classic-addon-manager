<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import * as Avatar from "$lib/components/ui/avatar/index";
    import {Button} from "$lib/components/ui/button";
    import {apiClient} from "../api";
    import {toast} from "../utils";
    import supportDaru from "../assets/images/support_daru.webp";
    import supportDaruAlt from "../assets/images/support_daru_alt_sm.webp";
    import DOMPurify from "dompurify";
    import {marked} from "marked";
    import type {User} from "$stores/UserStore.svelte";

    let { open = $bindable(false), user } = $props<{open?: boolean, user: User}>();

    let chatMessage: string = $state('');
    let chatHistory: { role: 'user' | 'assistant', content: string }[] = $state([]);
    let isWaitingForResponse: boolean = $state(false);
    let chatContainer: HTMLDivElement;
    let messageInput: HTMLInputElement;
    let lastMessageId: string = $state('');
    
    // Add new state variables for conversation tracking
    let conversationId: string | null = $state(null);
    let remainingLimit: number = $state(0);

    marked.setOptions({
        gfm: true,
        breaks: true,
    });

    // Configure DOMPurify to allow code blocks
    DOMPurify.addHook('afterSanitizeAttributes', function(node) {
        if (node.tagName === 'PRE' || node.tagName === 'CODE') {
            node.className = node.className || '';
        }
    });

    $effect(() => {
        if (chatHistory.length > 0) {
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
            }, 100);
        }
    });

    $effect(() => {
        if (!isWaitingForResponse && messageInput && open) {
            setTimeout(() => messageInput?.focus(), 0);
        }
    });

    async function sendMessage() {
        if (!chatMessage.trim() || isWaitingForResponse) return;

        // Add user message to chat history
        chatHistory = [...chatHistory, {role: 'user', content: chatMessage}];
        lastMessageId = `message-${chatHistory.length - 1}`;

        // Clear input field
        const userMessage = chatMessage;
        chatMessage = '';
        
        // Set loading state
        isWaitingForResponse = true;

        // Start timeout for slow response message
        let timeoutId: number | undefined = undefined;
        const timeoutPromise = new Promise((_, reject) => {
            timeoutId = setTimeout(() => {
                // Check if we are still waiting for the actual response
                if (isWaitingForResponse) {
                    chatHistory = [
                        ...chatHistory,
                        {
                            role: 'assistant',
                            content: "Sorry adventurer, locating the Daru merchants is taking longer than expected. I am still working on your question. 🐸"
                        }
                    ];
                    lastMessageId = `message-${chatHistory.length - 1}`;
                }
                // We don't reject here, just add the message
            }, 10000); // 10 seconds
        });

        try {
            // Create request payload, including conversation_id if available
            const payload = { 
                p: userMessage,
                ...(conversationId ? { conversation_id: conversationId } : {})
            };
            
            const response = await apiClient.post('/ai/chat', payload);
            if (response.ok) {
                const data = await response.json();
                
                // Store the conversation_id and remaining_limit
                conversationId = data.data.conversation_id || conversationId;
                remainingLimit = data.data.remaining_limit || remainingLimit;
                
                chatHistory = [...chatHistory, {
                    role: 'assistant',
                    content: data.data.response || 'Sorry, I could not process your request.'
                }];
                lastMessageId = `message-${chatHistory.length - 1}`;
            } else {
                throw new Error('Failed to get response from Daru');
            }
        } catch (error) {
            // If there's an error, add an error message to the chat
            chatHistory = [...chatHistory, {
                role: 'assistant',
                content: 'Sorry, I encountered an error processing your request.'
            }];
            lastMessageId = `message-${chatHistory.length - 1}`;
            console.error('Chat error:', error);
        } finally {
            // Clear the timeout regardless of success or failure
            if (timeoutId) {
                clearTimeout(timeoutId);
            }
            isWaitingForResponse = false;
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
    :global(.chat-message.assistant-message) {
        @apply prose prose-invert max-w-none;
        max-width: 100%;
        overflow-wrap: break-word;
        word-wrap: break-word;
        word-break: break-word;
    }

    :global(.chat-message.assistant-message a) {
        @apply text-blue-400 hover:text-blue-500 hover:underline transition-all;
    }

    :global(.chat-message.assistant-message code) {
        @apply bg-gray-700 py-0.5 px-1 rounded-sm font-mono;
        word-break: break-all;
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
    }

    :global(.chat-message.assistant-message ul) {
        @apply list-disc list-inside my-2;
    }

    :global(.chat-message.assistant-message ol) {
        @apply list-decimal list-inside my-2;
    }

    .copy-button {
        @apply absolute top-1 right-1 p-1 rounded-md text-muted-foreground/60 hover:text-muted-foreground hover:bg-secondary/50 transition-colors duration-150;
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
                                {#each chatHistory as message, i (message.content + i)}
                                    <div id={`message-${i}`} class="flex items-start gap-3 px-4 {message.role === 'user' ? 'justify-end' : ''}">
                                        {#if message.role === 'assistant'}
                                            <div class="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                                                <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                            </div>
                                        {/if}
                                        <div class="flex flex-col gap-2 {message.role === 'assistant' ? 'max-w-[calc(100%-3.5rem)] w-full' : 'max-w-[80%]'}">
                                            <div class="relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message {message.role === 'assistant' ? 'assistant-message' : ''}
                                                {message.role === 'user'
                                                    ? 'bg-primary text-black'
                                                    : 'bg-muted/50'}"
                                                 style="{message.role === 'assistant' ? 'max-width: calc(100% - 1rem); width: 100%;' : 'max-width: 100%;'}"
                                            >
                                                {#if message.role === 'assistant'}
                                                    {@html DOMPurify.sanitize(marked.parse(message.content, {async: false}))}
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
                                            <div class="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-secondary">
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
                                {/each}
                                
                                {#if isWaitingForResponse}
                                    <div class="flex items-start gap-3 px-4">
                                        <div class="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                                            <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                        </div>
                                        <div class="flex flex-col gap-2">
                                            <div class="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50">
                                                <div class="flex gap-1.5 items-center min-w-[2rem] min-h-[1.25rem]">
                                                    <span class="w-1.5 h-1.5 bg-current rounded-full animate-bounce-dot-1"></span>
                                                    <span class="w-1.5 h-1.5 bg-current rounded-full animate-bounce-dot-2"></span>
                                                    <span class="w-1.5 h-1.5 bg-current rounded-full animate-bounce-dot-3"></span>
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
                                placeholder={isWaitingForResponse ? "Consulting the Daru merchants..." : "Ask your question, the Darus won't judge..."}
                                disabled={isWaitingForResponse}
                                autofocus
                                class="w-full rounded-full border bg-muted/30 px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50 pr-10 disabled:opacity-50"
                            />
                            <Button
                                type="submit"
                                size="icon"
                                class="absolute right-1.5 top-1/2 -translate-y-1/2 transform rounded-full p-1.5 hover:bg-primary/10 transition-colors duration-200 disabled:opacity-50"
                                variant="ghost"
                                disabled={!chatMessage.trim() || isWaitingForResponse}
                            >
                                {#if isWaitingForResponse}
                                    <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
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
                    <p class="mt-2 text-center text-xs text-muted-foreground/80">
                        Darus are known for their wisdom, but sometimes even they make mistakes.
                    </p>
                </div>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>