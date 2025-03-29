<script lang="ts">
    import {Button} from "$lib/components/ui/button/index.js";
    import {setActiveScreen} from "../stores/ScreenStore.svelte";
    import BrowseAddonsScreen from "./BrowseAddonsScreen.svelte";
    import Search from "lucide-svelte/icons/search";
    import Refresh from "lucide-svelte/icons/refresh-cw";
    import Package from "lucide-svelte/icons/package";
    import {Input} from "$lib/components/ui/input/index.js";
    import {setActiveNavbar} from "$stores/NavbarStore.svelte";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import {getInstalledAddons} from "$stores/AddonStore.svelte";
    import type {Addon} from "$lib/wails";
    import {setUpdatesAvailableCount} from "$stores/AddonStore.svelte";
    import LocalAddon from "../components/LocalAddon.svelte";
    import {onMount} from "svelte";
    import { fade, fly } from 'svelte/transition';

    let searchName: string = $state("");
    let localAddons: Array<Addon> = $state([]);
    let isCheckingForUpdates = $state(false);
    let isLoading = $state(true);

    onMount(() => {
        setUpdatesAvailableCount(0);
    });

    $effect(() => {
        isLoading = true;
        localAddons = getInstalledAddons();
        isLoading = false;
    });

    let filteredAddons = $derived.by(() => {
        return localAddons.filter((addon) => {
            if (searchName === "") {
                return true;
            }
            return addon.name.toLowerCase().includes(searchName.toLowerCase());
        });
    });

    function checkForUpdates(): void {
        if (isCheckingForUpdates) {
            return;
        }
        setUpdatesAvailableCount(0);
        isCheckingForUpdates = true;
        const event = new CustomEvent("check-updates");
        document.dispatchEvent(event);
        setTimeout(() => {
            isCheckingForUpdates = false;
        }, 1000);
    }
</script>

<div class="flex flex-col h-screen bg-background">
    <header class="sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
        <div class="container flex h-16 items-center gap-4 px-4">
            <div class="flex-1">
                <div class="relative w-full">
                    <Search
                        class="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2"
                    />
                    <Input
                        type="search"
                        placeholder="Search installed addons..."
                        class="pl-10 pr-4 w-full transition-colors focus-visible:ring-1"
                        bind:value={searchName}
                        disabled={isLoading}
                    />
                </div>
            </div>
            <div class="flex items-center justify-center">
                <Button 
                    variant="outline" 
                    onclick={checkForUpdates} 
                    disabled={isCheckingForUpdates || isLoading}
                    class="transition-all duration-200 hover:shadow-md min-w-[160px]"
                >
                    <div class="flex items-center justify-center">
                        {#if isCheckingForUpdates}
                            <LoaderCircle class="mr-2 size-4 animate-spin"/>
                            <span>Checking...</span>
                        {:else}
                            <Refresh class="mr-2 size-4"/>
                            <span>Check Updates</span>
                        {/if}
                    </div>
                </Button>
            </div>
        </div>
    </header>

    <main class="flex-1 container px-4 py-6">
        <div class="flex items-center justify-between mb-6">
            <h1 class="text-3xl font-bold tracking-tight">Dashboard</h1>
            {#if !isLoading}
                <div class="text-sm text-muted-foreground">
                    {filteredAddons.length} {filteredAddons.length === 1 ? 'addon' : 'addons'} installed
                </div>
            {/if}
        </div>

        {#if isLoading}
            <div class="flex flex-col items-center justify-center rounded-lg border bg-card/50 p-8 text-center min-h-[400px]">
                <LoaderCircle class="size-8 animate-spin text-primary mb-4"/>
                <p class="text-muted-foreground">Loading addons...</p>
            </div>
        {:else if localAddons.length > 0}
            <div class="rounded-lg border bg-card shadow-sm" transition:fade>
                <div class="grid grid-cols-4 w-full p-2 border-b bg-muted/40">
                    <div class="font-medium text-sm text-muted-foreground">Name</div>
                    <div class="text-center font-medium text-sm text-muted-foreground">Author</div>
                    <div class="text-center font-medium text-sm text-muted-foreground">Version</div>
                    <div class="text-center font-medium text-sm text-muted-foreground">Status</div>
                </div>

                <div class="divide-y">
                    {#each filteredAddons as addon (addon.name)}
                        <div transition:fly={{ y: 20, duration: 300 }}>
                            <LocalAddon {addon}/>
                        </div>
                    {/each}
                </div>
            </div>
        {:else}
            <div class="flex flex-col items-center justify-center rounded-lg border bg-card/50 p-8 text-center min-h-[400px]" transition:fade>
                <div class="rounded-full bg-primary/10 p-4 mb-4">
                    <Package class="size-8 text-primary"/>
                </div>
                <h3 class="text-2xl font-semibold tracking-tight mb-2">
                    No Addons Installed
                </h3>
                <p class="text-muted-foreground text-sm max-w-sm mb-6">
                    Get started by browsing our collection of addons and enhance your ArcheAge experience.
                </p>
                <Button
                    size="lg"
                    onclick={() => {
                        setActiveScreen(BrowseAddonsScreen);
                        setActiveNavbar("addons");
                    }}
                    class="transition-all duration-200 hover:shadow-md"
                >
                    Browse Addons
                </Button>
            </div>
        {/if}
    </main>
</div>
