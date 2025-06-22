<script lang="ts">
    import * as Avatar from "$lib/components/ui/avatar/index";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index";
    import {Button} from "$lib/components/ui/button";
    import {clearUserState, getUser, isAuthenticated, setToken, setUser, type User} from "$stores/UserStore.svelte";
    import {onMount} from "svelte";
    import {apiClient} from "../api";
    import {toast} from "../utils";
    import {Events, Browser} from "@wailsio/runtime";
    import type {WailsEvent} from "node_modules/@wailsio/runtime/types/events";
    import addons from "../addons";
    import AIChatDialog from "./AIChatDialog.svelte";

    let isReady: boolean = $state(false);
    let user: User = $derived(getUser());
    let chatOpen: boolean = $state(false);

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
    }
</script>

<style>
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

<AIChatDialog bind:open={chatOpen} {user} />
