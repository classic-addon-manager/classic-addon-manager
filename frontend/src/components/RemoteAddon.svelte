<script lang="ts">
    import type {addon as ad} from "../../wailsjs/go/models.js";
    import {Skeleton} from "$lib/components/ui/skeleton";
    import {Button} from "$lib/components/ui/button";
    import {toast} from "svelte-sonner";
    import {onMount} from "svelte";
    import {IsAddonInstalled} from "../../wailsjs/go/main/App";
    import Blocks from "lucide-svelte/icons/blocks";
    import RemoteAddonDialog from "./remote_addon/RemoteAddonDialog.svelte";

    let {addon}: { addon: ad.AddonManifest } = $props();
    let isInstalled = $state(false);
    let openDialog = $state(false);
    let hasIcon = $state(false);
    let icon: string = $state('');

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

<div class="flex items-center justify-around bg-muted/50 hover:bg-gray-400/20 aspect-video h-12 w-full rounded-lg cursor-pointer transition-all"
     onclick={() => openDialog = true}
>
    <div class="grid grid-cols-10 items-center justify-between bg-muted/50 aspect-video h-12 w-full rounded-lg">
        <div class="flex gap-3 items-center col-span-4 overflow-hidden">
            {#if hasIcon}
                <img class="ml-2 h-[38px] w-[38px] rounded-sm" src={icon} alt=""/>
            {:else}
                <Blocks class="scale-75 opacity-50 flex-shrink-0 ml-2 h-[38px] w-[38px]"/>
            {/if}
            <div class="text-foreground font-medium- whitespace-nowrap">{addon.alias}</div>
        </div>
        <div class="text-foreground font-light col-span-2">{addon.author}</div>
        <div class="text-foreground font-light col-span-2">{addon.tags.join(', ')}</div>
        <div class="text-foreground col-span-1 ml-7">
            {#if isInstalled}
                <Button
                        variant="default"
                        class="w-24 h-8 cursor-not-allowed"
                        disabled={true}
                >
                    Installed
                </Button>
            {:else}
                <Button
                        variant="default"
                        class="w-24 h-8"
                >
                    Install
                </Button>
            {/if}
        </div>
    </div>
</div>