<script lang="ts">
    import {onDestroy, onMount} from "svelte";
    import {addon as ad, api} from "../../wailsjs/go/models";
    import {addUpdateAvailableCount,} from "$stores/AddonStore.svelte";
    import {GetLatestAddonRelease as GoGetLatestRelease} from "../../wailsjs/go/main/App";
    import LocalAddonContextMenu from "./local_addon/LocalAddonContextMenu.svelte";
    import LocalAddonDialog from "./local_addon/LocalAddonDialog.svelte";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import Download from "lucide-svelte/icons/download";
    import CheckMark from "lucide-svelte/icons/check";
    import ShieldQuestion from "lucide-svelte/icons/shield-question";
    import {Badge} from "$lib/components/ui/badge";
    import LocalAddonUpdateDialog from "./local_addon/LocalAddonUpdateDialog.svelte";

    interface Props {
        addon: ad.Addon;
    }

    let {addon}: Props = $props();

    let latestRelease: api.Release | undefined = $state();
    let isCheckingForUpdates = $state(false);
    let openDialog = $state(false);
    let openUpdateDialog = $state(false);

    function handleOpenDialogChange(o: boolean) {
        openDialog = o;
    }

    function handleOpenUpdateDialogChange(o: boolean) {
        openUpdateDialog = o;
    }

    async function handleCheckUpdates() {
        isCheckingForUpdates = true;
        console.log("Checking for updates for addon: ", addon.name);
        let release: api.Release | undefined;
        try {
            release = await GoGetLatestRelease(addon.name);
        } catch (e) {
            console.error("Failed to get release for addon: ", addon.name);
            isCheckingForUpdates = false;
            return;
        }
        if (!release) {
            console.error("No release found for addon: ", addon.name);
            return;
        }
        latestRelease = release;
        isCheckingForUpdates = false;

        if (release.published_at > addon.updatedAt) {
            addUpdateAvailableCount();
        }
    }

    onMount(async () => {
        if (addon.isManaged) {
            document.addEventListener("check-updates", handleCheckUpdates);
            await handleCheckUpdates();
        }

        console.log(addon);
    });

    onDestroy(() => {
        document.removeEventListener("check-updates", handleCheckUpdates);
    });
</script>

<LocalAddonDialog
        {addon}
        bind:open={openDialog}
        onOpenChange={handleOpenDialogChange}
/>

{#if latestRelease}
    <LocalAddonUpdateDialog
            {addon}
            release={latestRelease}
            bind:open={openUpdateDialog}
            onOpenChange={handleOpenUpdateDialogChange}
    />
{/if}

{#snippet contextTriggerArea()}
    <div class="grid grid-cols-4 p-2 hover:bg-muted/50 border-t transition-colors items-center text-sm"
         onclick={() => (openDialog = true)}>
        <div class="font-medium">{addon.alias}</div>
        <div class="text-center">{addon.author}</div>
        <div class="text-center">{addon.version}</div>
        <div class="text-center">
            {#if addon.isManaged}
                {#if isCheckingForUpdates}
                    <div class="flex justify-center">
                        <LoaderCircle size={20} class="mr-1 animate-spin"/>
                    </div>
                {:else if latestRelease}
                    {#if latestRelease.published_at > addon.updatedAt}
                        <Badge class="py-1 cursor-pointer" variant="default" onclick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            openUpdateDialog = true;
                            // TODO: Call update function
                        }}>
                            <Download size={14} class="mr-1"/>
                            Update ({latestRelease.tag_name})
                        </Badge>
                    {:else}
                        <div class="flex justify-center">
                            <CheckMark size={20} class="text-green-600 mr-2"/>
                            Up to
                            date
                        </div>
                    {/if}
                {:else}
                    <div class="flex justify-center">
                        <CheckMark size={20} class="text-green-600 mr-2"/>
                        Up to date
                    </div>
                {/if}
            {:else}
                <Badge class="py-1 cursor-pointer" variant="warning">
                    <ShieldQuestion size={14} class="mr-2"/>
                    Not managed
                </Badge
                >
            {/if}
        </div>
    </div>
{/snippet}

<LocalAddonContextMenu {addon} {contextTriggerArea}/>