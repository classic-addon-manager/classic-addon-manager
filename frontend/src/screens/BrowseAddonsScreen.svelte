<script lang="ts">
    import Search from "lucide-svelte/icons/search";
    import {Input} from "$lib/components/ui/input/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import {onMount} from "svelte";
    import type {AddonManifest} from "$lib/wails";
    import {RemoteAddonService} from "$lib/wails";
    import RemoteAddon from "../components/RemoteAddon.svelte";
    import RemoteAddonSkeleton from "../components/remote_addon/RemoteAddonSkeleton.svelte";
    import {Button} from "$lib/components/ui/button";
    import {timeAgo, toast} from "../utils";

    let isReady: boolean = $state(false);
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
            // @ts-ignore
            return addon.alias.toLowerCase().includes(searchPhrase.toLowerCase()) || addon.description.toLowerCase().includes(searchPhrase.toLowerCase());
        });
    });

    onMount(async () => {
        await getAddonManifest();
        isReady = true;
    });

    async function getAddonManifest() {
        let tmp: AddonManifest[] = [];
        for (let addon of await RemoteAddonService.GetAddonManifest()) {
            // @ts-ignore
            tmp.push({
                ...addon
            });
            for (let tag of addon.tags) {
                // Skip the Example tag
                if (tag == 'Example') {
                    continue;
                }
                if (tags.find(t => t.value == tag)) {
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
                // Both are new, sort by how new they are (smaller timeAgo = newer)
                const aTime = timeAgo(a.added_at);
                const bTime = timeAgo(b.added_at);
                if (aTime !== bTime) {
                    return aTime - bTime;
                }
            }

            // If not new or same newness, sort alphabetically
            return a.name.localeCompare(b.name);
        });
        tags.sort((a, b) => a.label.localeCompare(b.label));
        addons = tmp;
        searchPhrase = '';
    }
</script>

<header class="bg-muted/40 flex h-14 items-center gap-4 border-b px-4">
    <div class="flex gap-2 w-full">
        <div class="relative flex-1">
            <Search
                    class="text-muted-foreground absolute left-2.5 top-2.5 h-4 w-4"
            />
            <Input
                    disabled={!isReady}
                    type="search"
                    placeholder="Search addons..."
                    class="bg-background w-full appearance-none pl-8 shadow-none"
                    bind:value={searchPhrase}
            />
        </div>
        <div class="ml-auto">
            <Select.Root type="single" bind:value={selectedTag} allowDeselect={false} disabled={!isReady}>
                <Select.Trigger class="w-[180px]">
                    {triggerContent}
                </Select.Trigger>
                <Select.Content>
                    {#each tags as tag}
                        <Select.Item value={tag.value} label={tag.label}/>
                    {/each}
                </Select.Content>
            </Select.Root>
        </div>
        <div>
            <Button
                    variant="secondary"
                    disabled={!isReady}
                    onclick={async() => {
                        await getAddonManifest();
                        toast.success('Addons refreshed');
                    }}
            >
                Refresh
            </Button>
        </div>
    </div>
</header>

<main class=" h-[calc(100vh-4rem)] overflow-auto">
    <div class="pt-4 px-5 grid grid-cols-10 w-full p-2 text-muted-foreground font-medium">
        <div class="font-semibold text-sm col-span-4">
            <h3>Name</h3>
        </div>
        <div class="font-semibold mr-auto text-sm col-span-2">
            <h3>Author</h3>
        </div>
        <div class="font-semibold mr-auto text-sm col-span-2">
            <h3>Tags</h3>
        </div>
        <div class="font-semibold text-sm text-center col-span-2">
            <h3>Status</h3>
        </div>
    </div>
    {#if !isReady}
        <div class="flex flex-1 flex-col gap-4 px-4 pb-4">
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
            <RemoteAddonSkeleton/>
        </div>
    {:else}
        <div class="flex flex-1 flex-col gap-4 px-4 pb-4">
            {#each filteredAddons as addon(addon.name)}
                <RemoteAddon {addon}/>
            {/each}
        </div>
    {/if}
</main>
