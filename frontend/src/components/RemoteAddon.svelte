<script lang="ts">
    import type {addon as ad} from "../../wailsjs/go/models.js";
    import addons from "../addons";
    import {Skeleton} from "$lib/components/ui/skeleton";
    import {Button} from "$lib/components/ui/button";
    import {toast} from "svelte-sonner";
    import {onMount} from "svelte";
    import {IsAddonInstalled} from "../../wailsjs/go/main/App";
    import Blocks from "lucide-svelte/icons/blocks";
    import RemoteAddonDialog from "./remote_addon/RemoteAddonDialog.svelte";

    let {addon}: { addon: ad.AddonManifest } = $props();
    let isInstalled = $state(false);
    let isDownloading = $state(false);
    let openDialog = $state(false);

    onMount(async () => {
        isInstalled = await IsAddonInstalled(addon.name);
    });

    function handleOpenDialogChange(o: boolean) {
        openDialog = o;
    }

    async function handleInstallClick() {
        if (isInstalled || isDownloading) return;
        isDownloading = true;

        try {
            isInstalled = await addons.install(addon)
        } catch (e) {
            if (e == 'no release found') {
                toast.error(`No release found for ${addon.name}`);
                isDownloading = false;
                isInstalled = false;
                return;
            }
            isInstalled = false;
        }
        isDownloading = false;
    }

    function handleOnInstall(installed: boolean): void {
        isInstalled = installed;
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
            <Blocks class="scale-75 opacity-50 flex-shrink-0 ml-2 h-[38px] w-[38px]"/>
            <!--            <img src={doodleru} alt="Doodleru" class="scale-90 opacity-50 flex-shrink-0 ml-2 h-[38px] w-[38px]"/>-->
            <!--            <Skeleton class="flex-shrink-0 ml-2 h-[38px] w-[38px]"/>-->
            <div class="text-foreground font-medium- whitespace-nowrap">{addon.alias}

            </div>
        </div>
        <div class="text-foreground font-light col-span-2">{addon.author}</div>
        <div class="text-foreground font-light col-span-2">{addon.tags.join(', ')}</div>
        <div class="text-foreground col-span-1 ml-7">
            {#if isInstalled}
                <Button
                        variant="default"
                        class="w-24 h-8 cursor-not-allowed"
                        disabled={true}
                        onclick={handleInstallClick}
                >
                    Installed
                </Button>
            {:else}
                <Button
                        variant="default"
                        class="w-24 h-8"
                        onclick={handleInstallClick}
                >
                    Install
                </Button>
            {/if}
        </div>
    </div>
</div>

<style>

</style>