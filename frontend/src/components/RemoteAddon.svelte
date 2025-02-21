<script lang="ts">
    import type {addon as ad} from "$lib/wails"
    import {Button} from "$lib/components/ui/button";
    import {onMount} from "svelte";
    import {IsAddonInstalled} from "$lib/wails";
    import Blocks from "lucide-svelte/icons/blocks";
    import Heart from "lucide-svelte/icons/heart";
    import Download from "lucide-svelte/icons/download";
    import Check from "lucide-svelte/icons/check";

    import RemoteAddonDialog from "./remote_addon/RemoteAddonDialog.svelte";
    import {timeAgo} from "../utils";

    let {addon}: { addon: ad.AddonManifest } = $props();
    let isInstalled = $state(false);
    let openDialog = $state(false);
    let hasIcon = $state(false);
    let icon: string = $state('');
    let isNew = timeAgo(addon.added_at) < 32;

    onMount(async () => {
        isInstalled = await IsAddonInstalled(addon.name);
        hasIcon = await checkForIcon();
    });

    function handleOpenDialogChange(o: boolean) {
        openDialog = o;
    }

    function handleOnInstall(installed: boolean): void {
        isInstalled = installed;
    }

    async function checkForIcon() {
        const response = await fetch(`https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`);
        if (!response.ok) {
            return false;
        }

        const reader = response.body?.getReader();
        if (reader) {
            const chunks = [];
            let done = false;
            while (!done) {
                let result;
                try {
                    result = await reader.read();
                } catch (e) {
                    console.error("Error reading image:", e);
                    return false;
                }
                done = result.done;
                if (result.value) {
                    chunks.push(result.value);
                }
            }
            const blob = new Blob(chunks);
            icon = URL.createObjectURL(blob);
        }
        return true;
    }
</script>

<RemoteAddonDialog
        bind:open={openDialog}
        {addon}
        onOpenChange={handleOpenDialogChange}
        onInstall={handleOnInstall}
/>

<div class="flex items-center justify-around bg-muted/50 hover:bg-gray-400/20 aspect-video h-16 w-full rounded-lg cursor-pointer transition-all"
     onclick={() => openDialog = true}
>
    <div class="grid grid-cols-10 items-center justify-between bg-muted/50 aspect-video h-16 w-full rounded-lg">
        <div class="flex gap-3 items-center col-span-4 fade-end overflow-hidden">
            <div class="relative">
                {#if hasIcon}
                    <img class="ml-2 h-[38px] w-[38px] rounded-sm" src={icon} alt=""/>
                {:else}
                    <Blocks class="scale-75 opacity-50 flex-shrink-0 ml-2 h-[38px] w-[38px]"/>
                {/if}

                {#if isNew}
                    <div class="opacity-90 absolute bottom-7 left-7 bg-red-500 text-white text-[9px] font-bold px-0.5 py-0.5 rounded">
                        NEW
                    </div>
                {/if}
            </div>


            <div class="flex-col">
                <div class="text-foreground whitespace-nowrap">{addon.alias}</div>
                <div class="flex items-center gap-1 text-muted-foreground">
                    <Download class="w-4 scale-90"/> {addon.downloads}

                    {#if addon.like_percentage !== null}
                        <Heart class="w-4 scale-90"/> {addon.like_percentage}%
                    {/if}
                </div>
            </div>
        </div>

        <div class="text-foreground font-light col-span-2">{addon.author}</div>
        <div class="text-foreground font-light col-span-2">{addon.tags.join(', ')}</div>
        <div class="text-foreground col-span-2 ml-7">
            <div class="flex gap-4">
                {#if isInstalled}
                    <Button
                            variant="default"
                            class="w-20 h-8 cursor-not-allowed"
                            disabled={true}
                    >
                        <Check/>
                    </Button>
                {:else}
                    <Button
                            variant="default"
                            class="w-20 h-8"
                    >
                        <Download/>
                    </Button>
                {/if}
            </div>
        </div>
    </div>
</div>