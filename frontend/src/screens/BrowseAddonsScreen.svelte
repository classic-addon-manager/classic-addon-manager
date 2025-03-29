<script lang="ts">
    import Search from "lucide-svelte/icons/search";
    import RefreshCw from "lucide-svelte/icons/refresh-cw";
    import PackageSearch from "lucide-svelte/icons/package-search";
    import {Input} from "$lib/components/ui/input/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import {onMount} from "svelte";
    import type {AddonManifest} from "$lib/wails";
    import {RemoteAddonService} from "$lib/wails";
    import RemoteAddon from "../components/RemoteAddon.svelte";
    import RemoteAddonSkeleton from "../components/remote_addon/RemoteAddonSkeleton.svelte";
    import {Button} from "$lib/components/ui/button";
    import {timeAgo, toast} from "../utils";
    import {fade, fly} from 'svelte/transition';

    let isReady: boolean = $state(false);
    let isRefreshing: boolean = $state(false);
    let searchPhrase: string = $state('');
    let addons: AddonManifest[] = $state([]);
    let tags = [
        {label: 'All', value: 'all'},
    ];

    let selectedTag: string = $state('all');
    const triggerContent = $derived(
        tags.find((t) => t.value === selectedTag)?.label ?? "Select a tag"
    );

    const filteredAddons = $derived.by(() => {
        return addons.filter(addon => {
            if (selectedTag != 'all' && !addon.tags.includes(selectedTag)) {
                return false;
            }
            return addon.alias.toLowerCase().includes(searchPhrase.toLowerCase()) || addon.description.toLowerCase().includes(searchPhrase.toLowerCase());
        });
    });

    onMount(async () => {
        await loadAddons();
        isReady = true;
    });

    async function loadAddons() {
        let tmp: AddonManifest[] = [];
        for (let addon of await RemoteAddonService.GetAddonManifest()) {
            tmp.push({
                ...addon
            });
            for (let tag of addon.tags) {
                if (tag == 'Example' || tags.find(t => t.value == tag)) {
                    continue;
                }
                tags.push({label: tag, value: tag});
            }
        }

        // Sort addons alphabetically by name and whether an addon is new or not
        tmp.sort((a, b) => {
            const aIsNew = timeAgo(a.added_at) < 32;
            const bIsNew = timeAgo(b.added_at) < 32;

            if (aIsNew && !bIsNew) return -1;
            if (!aIsNew && bIsNew) return 1;

            if (aIsNew && bIsNew) {
                const aTime = timeAgo(a.added_at);
                const bTime = timeAgo(b.added_at);
                if (aTime !== bTime) {
                    return aTime - bTime;
                }
            }

            return a.name.localeCompare(b.name);
        });
        tags.sort((a, b) => a.label.localeCompare(b.label));
        addons = tmp;
        searchPhrase = '';
    }

    async function getAddonManifest() {
        if (isRefreshing) return;
        
        isRefreshing = true;
        const startTime = Date.now();
        
        try {
            await loadAddons();
            
            // Only add artificial delay when manually refreshing
            const elapsedTime = Date.now() - startTime;
            if (elapsedTime < 500) {
                await new Promise(resolve => setTimeout(resolve, 500 - elapsedTime));
            }
            
            toast.success('Addons refreshed');
        } finally {
            isRefreshing = false;
        }
    }
</script>

<div class="flex flex-col h-screen">
    <header class="bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
        <div class="container flex h-16 items-center gap-4 px-4">
            <div class="relative flex-1">
                <Search
                    class="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform"
                />
                <Input
                    disabled={!isReady}
                    type="search"
                    placeholder="Search addons..."
                    class="bg-background w-full pl-10 shadow-none transition-colors focus-visible:bg-background/80"
                    bind:value={searchPhrase}
                />
            </div>
            <div class="flex items-center gap-3 shrink-0">
                <Select.Root type="single" bind:value={selectedTag} allowDeselect={false} disabled={!isReady}>
                    <Select.Trigger class="w-[140px] bg-background/80">
                        {triggerContent}
                    </Select.Trigger>
                    <Select.Content>
                        {#each tags as tag}
                            <Select.Item value={tag.value} label={tag.label}/>
                        {/each}
                    </Select.Content>
                </Select.Root>
                <Button
                    variant="outline"
                    disabled={!isReady || isRefreshing}
                    class="min-w-[120px] w-[120px] flex items-center justify-center gap-2 transition-all duration-200 hover:shadow-md"
                    onclick={async() => {
                        await getAddonManifest();
                    }}
                >
                    <RefreshCw class="h-4 w-4 {isRefreshing ? 'animate-spin' : ''}" />
                    {isRefreshing ? 'Refreshing' : 'Refresh'}
                </Button>
            </div>
        </div>
    </header>

    <main class="flex-1 overflow-auto">
        <div class="container px-4">
            <div class="grid grid-cols-10 w-full py-4 text-muted-foreground font-medium border-b sticky top-0 bg-background z-20">
                <div class="font-semibold text-sm col-span-4">
                    <h3>Name</h3>
                </div>
                <div class="font-semibold text-sm col-span-2">
                    <h3>Author</h3>
                </div>
                <div class="font-semibold text-sm col-span-2">
                    <h3>Tags</h3>
                </div>
                <div class="font-semibold text-sm text-center col-span-2">
                    <h3>Status</h3>
                </div>
            </div>

            {#if !isReady}
                <div class="flex flex-1 flex-col gap-4 py-4 relative" in:fade>
                    <RemoteAddonSkeleton/>
                    <RemoteAddonSkeleton/>
                    <RemoteAddonSkeleton/>
                    <RemoteAddonSkeleton/>
                    <RemoteAddonSkeleton/>
                </div>
            {:else if filteredAddons.length === 0}
                <div class="flex flex-col items-center justify-center py-16 text-center text-muted-foreground" in:fade>
                    <PackageSearch class="h-12 w-12 mb-4" />
                    <h3 class="text-lg font-semibold mb-2">No addons found</h3>
                    <p class="max-w-sm">
                        {#if searchPhrase}
                            No addons match your search criteria. Try adjusting your search or filters.
                        {:else}
                            No addons are currently available. Check back later or try refreshing.
                        {/if}
                    </p>
                </div>
            {:else}
                <div class="flex flex-1 flex-col gap-4 py-4">
                    {#each filteredAddons as addon(addon.name)}
                        <div in:fly={{y: 20, duration: 200}}>
                            <RemoteAddon {addon}/>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </main>
</div>

<style>
    /* Add smooth transitions */
    :global(.transition-colors) {
        transition-property: background-color, border-color, color, fill, stroke;
        transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
        transition-duration: 150ms;
    }
</style>
