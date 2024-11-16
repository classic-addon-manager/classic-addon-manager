<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { addon as ad, api } from "../../wailsjs/go/models";
    import { addUpdateAvailableCount } from "$stores/AddonStore.svelte";
    import { GetLatestRelease as GoGetLatestRelease } from "../../wailsjs/go/main/App";

    import * as Table from "$lib/components/ui/table/index.js";
    import CheckMark from "lucide-svelte/icons/check";
    import Download from "lucide-svelte/icons/download";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import ShieldQuestion from "lucide-svelte/icons/shield-question";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import LocalAddonContextMenu from "./addon/LocalAddonContextMenu.svelte";

    interface Props {
        addon: ad.Addon;
    }

    let { addon }: Props = $props();

    let latestRelease: api.Release | undefined = $state();
    let isCheckingForUpdates = $state(false);

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
    });

    onDestroy(() => {
        document.removeEventListener("check-updates", handleCheckUpdates);
    });
</script>

<!-- 
    In order to not break tables the context menu has to be applied to every element.  
    If you are just starting out looking to contribute to the project, this is a good place to start.
-->
<Table.Row class="cursor-pointer">
    <Table.Cell class="font-medium">
        <LocalAddonContextMenu addonData={addon}>
            {#snippet content()}
                {addon.displayName}
            {/snippet}
        </LocalAddonContextMenu>
    </Table.Cell>
    <Table.Cell class="text-center">
        <LocalAddonContextMenu addonData={addon}>
            {#snippet content()}
                {addon.author || "?"}
            {/snippet}
        </LocalAddonContextMenu>
    </Table.Cell>
    <Table.Cell class="text-center">
        <LocalAddonContextMenu addonData={addon}>
            {#snippet content()}
                {addon.version || "?"}
            {/snippet}
        </LocalAddonContextMenu>
    </Table.Cell>
    <Table.Cell class="text-center">
        {#if addon.isManaged}
            {#if isCheckingForUpdates}
                <div class="flex justify-center">
                    <LoaderCircle size={20} class="mr-1 animate-spin" />
                </div>
            {:else if latestRelease}
                {#if latestRelease.published_at > addon.updatedAt}
                    <Badge class="py-1 cursor-pointer" variant="default">
                        <Download size={14} class="mr-1" /> Update ({latestRelease.tag_name})
                    </Badge>
                {:else}
                    <div class="flex justify-center">
                        <CheckMark size={20} class="text-green-600 mr-2" /> Up to
                        date
                    </div>
                {/if}
            {:else}
                <div class="flex justify-center">
                    <CheckMark size={20} class="text-green-600 mr-2" /> Up to date
                </div>
            {/if}
        {:else}
            <Badge class="py-1 cursor-pointer" variant="warning">
                <ShieldQuestion size={20} class="mr-2" />
                Not managed</Badge
            >
        {/if}
    </Table.Cell>
</Table.Row>
