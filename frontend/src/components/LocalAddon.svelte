<script lang="ts">
    import {onDestroy, onMount} from "svelte";
    import type {Addon, Release} from "$lib/wails";
    import {addUpdateAvailableCount} from "$stores/AddonStore.svelte";
    import {RemoteAddonService} from "$lib/wails";
    import LocalAddonContextMenu from "./local_addon/LocalAddonContextMenu.svelte";
    import LocalAddonDialog from "./local_addon/LocalAddonDialog.svelte";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import Download from "lucide-svelte/icons/download";
    import CheckMark from "lucide-svelte/icons/check";
    import ShieldQuestion from "lucide-svelte/icons/shield-question";
    import {Badge} from "$lib/components/ui/badge";
    import LocalAddonUpdateDialog from "./local_addon/LocalAddonUpdateDialog.svelte";
    import { safeCall } from "../utils";

    interface Props {
        addon: Addon;
    }

    let {addon}: Props = $props();

    let latestRelease: Release | undefined = $state();
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
        const [release, err] = await safeCall<Release|undefined>(RemoteAddonService.GetLatestRelease(addon.name));
        if(err) {
            console.error("Failed to get release for addon: ", addon.name);
            isCheckingForUpdates = false;
            return;
        }
        if (!release) {
            console.error("No release found for addon: ", addon.name);
            isCheckingForUpdates = false;
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
    <div
            class="cursor-pointer grid grid-cols-4 p-2 hover:bg-muted/50 border-t transition-colors items-center text-sm"
            onclick={() => (openDialog = true)}
    >
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
                        <Badge
                                class="py-1 cursor-pointer flex-shrink-0 flex-grow-0"
                                variant="default"
                                onclick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                openUpdateDialog = true;
                            }}
                        >
                            <Download size={14} class="mr-1"/>
                            Update ({latestRelease.tag_name})
                        </Badge>
                    {:else}
                        <div class="flex justify-center">
                            <CheckMark size={20} class="text-green-600 mr-2"/>
                            Up to date
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
                </Badge>
            {/if}
        </div>
    </div>
{/snippet}

<LocalAddonContextMenu {addon} {contextTriggerArea}/>
