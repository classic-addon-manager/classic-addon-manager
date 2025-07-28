<script lang="ts">
    import type {AddonManifest} from "$lib/wails";
    import {Button} from "$lib/components/ui/button";
    import Blocks from "lucide-svelte/icons/blocks";
    import Heart from "lucide-svelte/icons/heart";
    import Download from "lucide-svelte/icons/download";
    import Check from "lucide-svelte/icons/check";

    import {timeAgo} from "../utils";

    let {addon, isInstalled, onViewDetails}: {
        addon: AddonManifest,
        isInstalled: boolean,
        onViewDetails: () => void;
    } = $props();

    // We assume the icon exists and let the <img> `onerror` event tell us if it doesn't.
    let hasIcon = $state(true);
    let isNew = timeAgo(addon.added_at) < 32;

    const iconUrl = `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`;

</script>

<div class="grid grid-cols-12 items-center gap-2 bg-muted/50 hover:bg-muted h-16 w-full rounded-lg cursor-pointer transition-colors px-3 py-2 group"
     onclick={onViewDetails}
>
    <!-- Left Section: Icon, Title, Stats-->
    <div class="flex items-center gap-3 col-span-5">
        <div class="relative flex-shrink-0">
            {#if hasIcon}
                <img class="h-10 w-10 rounded-md object-cover"
                     src={iconUrl}
                     alt="{addon.alias} icon"
                     loading="lazy"
                     onerror={() => hasIcon = false}
                />
            {:else}
                <!-- This block is now the fallback when the image fails to load -->
                <div class="flex items-center justify-center h-10 w-10 rounded-md bg-background border">
                    <Blocks class="h-5 w-5 opacity-40"/>
                </div>
            {/if}
            {#if isNew}
                <div class="absolute -top-1.5 -right-1.5 bg-red-500 text-white text-[8px] font-bold px-1 py-0.5 rounded-full shadow-md">
                    NEW
                </div>
            {/if}
        </div>
        <div class="flex flex-col overflow-hidden">
            <!-- First Row: Alias -->
            <div class="text-foreground font-medium truncate group-hover:text-primary transition-colors">{addon.alias}</div>
            <!-- Second Row: Stats (Downloads/Likes) -->
            <div class="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                <div class="flex items-center gap-0.5" title="{addon.downloads} downloads">
                    <Download class="w-3 h-3"/> {addon.downloads}
                </div>
                {#if addon.like_percentage !== null}
                    <div class="flex items-center gap-0.5" title="{addon.like_percentage}% likes">
                        <Heart class="w-3 h-3"/> {addon.like_percentage}%
                    </div>
                {/if}
            </div>
        </div>
    </div>

    <!-- Middle Section 1: Author -->
    <div class="text-sm text-muted-foreground truncate text-center col-span-3" title="Author: {addon.author}">
        {addon.author}
    </div>

    <!-- Middle Section 2: Tags -->
    {#if addon.tags.length > 0}
        <div class="flex items-center justify-end gap-1 flex-wrap col-span-3">
            {#each addon.tags.slice(0, 4) as tag}
                <span class="text-xs bg-secondary text-secondary-foreground px-1.5 py-0.5 rounded whitespace-nowrap">{tag}</span>
            {/each}
            {#if addon.tags.length > 4}
                <span class="text-xs text-muted-foreground">+{addon.tags.length - 4} more</span>
            {/if}
        </div>
    {:else}
        <div class="col-span-3"></div>
    {/if}

    <!-- Right Section: Button -->
    <div class="flex justify-end col-span-1">
        {#if isInstalled}
            <Button
                    variant="ghost"
                    size="icon"
                    class="cursor-default text-green-500 focus-visible:ring-0 focus-visible:ring-offset-0"
                    disabled={true}
                    aria-label="Installed"
                    tabindex={-1}
            >
                <Check class="h-5 w-5"/>
            </Button>
        {:else}
            <Button
                    variant="ghost"
                    size="icon"
                    aria-label="View details and install"
                    class="text-muted-foreground hover:text-primary focus-visible:ring-primary/40"
            >
                <Download class="h-5 w-5"/>
            </Button>
        {/if}
    </div>
</div>