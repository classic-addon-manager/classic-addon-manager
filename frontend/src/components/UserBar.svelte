<script lang="ts">
    import * as Avatar from "$lib/components/ui/avatar/index";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index";
    import {Button} from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog/index";
    import {clearUserState, getUser, isAuthenticated, setToken, setUser, type User} from "$stores/UserStore.svelte";
    import {onMount} from "svelte";
    import {apiClient} from "../api";
    import {toast} from "../utils";
    import {Events, Browser} from "@wailsio/runtime";
    import type {WailsEvent} from "node_modules/@wailsio/runtime/types/events";
    import addons from "../addons";
    import supportDaru from "../assets/images/support_daru.webp";
    import supportDaruAlt from "../assets/images/support_daru_alt_sm.webp";
    import DOMPurify from "dompurify";
    import {marked} from "marked";

    let isReady: boolean = $state(false);
    let user: User = $derived(getUser());
    let chatOpen: boolean = $state(false);
    let chatMessage: string = $state('');
    let chatHistory: { role: 'user' | 'assistant', content: string }[] = $state([]);
    let isWaitingForResponse: boolean = $state(false);
    let chatContainer: HTMLDivElement;
    let messageInput: HTMLInputElement;

    marked.setOptions({
        gfm: true,
        breaks: true,
    });

    $effect(() => {
        if (chatHistory.length > 0) {
            setTimeout(() => {
                chatContainer?.scrollTo({
                    top: chatContainer.scrollHeight,
                    behavior: 'smooth'
                });
            }, 100);
        }
    });

    $effect(() => {
        if (!isWaitingForResponse && messageInput && chatOpen) {
            setTimeout(() => messageInput?.focus(), 0);
        }
    });

    onMount(async () => {
        Events.On('authTokenReceived', async (event: WailsEvent) => {
            const token = Array.isArray(event.data) ? event.data[0] : event.data;
            if (typeof token === 'string') {
                setToken(token);
                await getAccount();
                toast.success('Successfully signed in');
            } else {
                console.error('Received unexpected data type for auth token:', event.data);
                toast.error('Failed to process sign-in token.');
            }
        });
        await getAccount();
        isReady = true;
    });

    $effect(() => {
        if (isAuthenticated()) {
            addons.getSubscribedAddons();
        }
    });

    async function getAccount() {
        let token = localStorage.getItem('token');
        if (token && token != null) {
            setToken(token);
        } else {
            // No token, no user
            return;
        }

        const resp = await apiClient.get('/me');
        if (resp.status === 200) {
            let user = await resp.json();
            setUser(user);
        }
    }

    function handleSignOut() {
        clearUserState();
        setTimeout(() => {
            window.location.href = '/';
        }, 100);
    }

    function handleChatOpen() {
        chatOpen = true;
        setTimeout(() => messageInput?.focus(), 100);
    }

    function handleChatClose() {
        chatOpen = false;
    }

    async function sendMessage() {
        if (!chatMessage.trim() || isWaitingForResponse) return;

        // Add user message to chat history
        chatHistory = [...chatHistory, {role: 'user', content: chatMessage}];

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
                }
                // We don't reject here, just add the message
            }, 10000); // 10 seconds
        });

        try {
            const response = await apiClient.post('/daru/inquiry', { p: userMessage });
            if (response.ok) {
                const payload = await response.json();
                console.log(payload);
                chatHistory = [...chatHistory, {
                    role: 'assistant',
                    content: payload.data.response || 'Sorry, I could not process your request.'
                }];
            } else {
                throw new Error('Failed to get response from Daru');
            }
        } catch (error) {
            // If there's an error, add an error message to the chat
            chatHistory = [...chatHistory, {
                role: 'assistant',
                content: 'Sorry, I encountered an error processing your request.'
            }];
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
    }

    :global(.chat-message.assistant-message a) {
        @apply text-blue-400 hover:text-blue-500 hover:underline transition-all;
    }

    :global(.chat-message.assistant-message code) {
        @apply bg-gray-700 py-0.5 px-1 rounded-sm font-mono;
    }

    :global(.chat-message.assistant-message pre) {
        @apply bg-neutral-900 p-4 my-4 overflow-x-auto rounded-md font-mono text-muted-foreground;
    }

    :global(.chat-message.assistant-message pre code) {
        @apply bg-transparent p-0 rounded-none;
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

{#if isReady}
    {#if !isAuthenticated()}
        <div class="mx-auto w-full">
            <a class="flex scale-[85%] items-center py-2 px-4 rounded-lg bg-[#5865F2] hover:bg-[#5865F2]/80 hover:text-white/80 transition-colors duration-300"
               onclick={(e) => { e.preventDefault(); Browser.OpenURL('https://discord.com/oauth2/authorize?client_id=1331010099916836914&response_type=code&redirect_uri=https%3A%2F%2Faac.gaijin.dev%2Fauth%2Fdiscord%2Fcallback2&scope=identify'); }}
               href="#"
            >
                <svg viewBox="0 -28.5 256 256" class="h-7 w-7 fill-white hover:fill-white/80 mr-4">
                    <path
                            d="M216.856339,16.5966031 C200.285002,8.84328665 182.566144,3.2084988 164.041564,0 C161.766523,4.11318106 159.108624,9.64549908 157.276099,14.0464379 C137.583995,11.0849896 118.072967,11.0849896 98.7430163,14.0464379 C96.9108417,9.64549908 94.1925838,4.11318106 91.8971895,0 C73.3526068,3.2084988 55.6133949,8.86399117 39.0420583,16.6376612 C5.61752293,67.146514 -3.4433191,116.400813 1.08711069,164.955721 C23.2560196,181.510915 44.7403634,191.567697 65.8621325,198.148576 C71.0772151,190.971126 75.7283628,183.341335 79.7352139,175.300261 C72.104019,172.400575 64.7949724,168.822202 57.8887866,164.667963 C59.7209612,163.310589 61.5131304,161.891452 63.2445898,160.431257 C105.36741,180.133187 151.134928,180.133187 192.754523,160.431257 C194.506336,161.891452 196.298154,163.310589 198.110326,164.667963 C191.183787,168.842556 183.854737,172.420929 176.223542,175.320965 C180.230393,183.341335 184.861538,190.991831 190.096624,198.16893 C211.238746,191.588051 232.743023,181.531619 254.911949,164.955721 C260.227747,108.668201 245.831087,59.8662432 216.856339,16.5966031 Z M85.4738752,135.09489 C72.8290281,135.09489 62.4592217,123.290155 62.4592217,108.914901 C62.4592217,94.5396472 72.607595,82.7145587 85.4738752,82.7145587 C98.3405064,82.7145587 108.709962,94.5189427 108.488529,108.914901 C108.508531,123.290155 98.3405064,135.09489 85.4738752,135.09489 Z M170.525237,135.09489 C157.88039,135.09489 147.510584,123.290155 147.510584,108.914901 C147.510584,94.5396472 157.658606,82.7145587 170.525237,82.7145587 C183.391518,82.7145587 193.761324,94.5189427 193.539891,108.914901 C193.539891,123.290155 183.391518,135.09489 170.525237,135.09489 Z"
                            fill="currentColor"
                            fill-rule="nonzero"
                    ></path>
                </svg>
                <span class="text-sm font-semibold">Sign in with discord</span>
            </a>
        </div>

    {:else}
        <div class="w-full px-3 space-y-2">
            <!-- Ask Question Button -->
            <Button
                    variant="secondary"
                    class="w-full flex items-center justify-center gap-2 bg-secondary/30"
                    onclick={handleChatOpen}
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                     class="lucide lucide-message-circle-question">
                    <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z"/>
                    <path d="M10 8.5a2.5 2.5 0 0 1 4 2 2.5 2.5 0 0 1-2.5 2.5"/>
                    <path d="M12 16h.01"/>
                </svg>
                Ask a friendly Daru
            </Button>

            <!-- User Profile -->
            <DropdownMenu.Root>
                <DropdownMenu.Trigger class="w-full focus:outline-none">
                    <div class="flex w-full items-center space-x-3 rounded-md bg-secondary/30 p-2 transition-all hover:bg-secondary">
                        <Avatar.Root class="h-8 w-8">
                            <Avatar.Image
                                    src={`https://cdn.discordapp.com/avatars/${user.discord_id}/${user.avatar}.png`}
                                    alt={user.username}
                                    class="h-full w-full object-cover"
                            />
                            <Avatar.Fallback class="text-xs">
                                {user.username.substring(0, 2).toUpperCase()}
                            </Avatar.Fallback>
                        </Avatar.Root>
                        <span class="text-sm font-medium flex-1 text-left">{user.username}</span>
                    </div>
                </DropdownMenu.Trigger>
                <DropdownMenu.Content class="w-56">
                    <DropdownMenu.Group>
                        <DropdownMenu.Label>Account</DropdownMenu.Label>
                        <DropdownMenu.Separator/>
                        <DropdownMenu.Item>
                            <button
                                    class="w-full text-left cursor-pointer text-red-500"
                                    onclick={handleSignOut}
                            >
                                Sign out
                            </button>
                        </DropdownMenu.Item>
                    </DropdownMenu.Group>
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    {/if}
{/if}

<!-- Chat Dialog -->
<Dialog.Root bind:open={chatOpen}>
    <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
        <Dialog.Content
                class="fixed left-[50%] top-[50%] z-50 w-[calc(100%-2rem)] max-w-2xl translate-x-[-50%] translate-y-[-50%] gap-0 border bg-background shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-xl overflow-hidden">
            <div class="flex max-h-[calc(100vh-4rem)] h-[650px] flex-col">
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
                                    <div class="flex items-start gap-3 px-4 {message.role === 'user' ? 'justify-end' : ''}">
                                        {#if message.role === 'assistant'}
                                            <div class="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-primary/10 overflow-hidden">
                                                <img src={supportDaruAlt} alt="Daru Assistant" class="w-full h-full object-cover" />
                                            </div>
                                        {/if}
                                        <div class="flex flex-col gap-2">
                                            <div class="relative inline-block rounded-lg px-3 py-1.5 text-sm chat-message {message.role === 'assistant' ? 'assistant-message' : ''}
                                                {message.role === 'user' 
                                                    ? 'bg-primary text-black' 
                                                    : 'bg-muted/50'}"
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
