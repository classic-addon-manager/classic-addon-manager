<script lang="ts">
    import type {Snippet} from "svelte";
    import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
    import Trash from "lucide-svelte/icons/trash";
    import AlertTriangle from "lucide-svelte/icons/alert-triangle";
    import Bug from "lucide-svelte/icons/bug";
    import Github from "lucide-svelte/icons/github";
    import FolderOpen from "lucide-svelte/icons/folder-open";
    import addons from "../../addons";
    import {LocalAddonService, type Addon} from "$lib/wails";
    import {toast} from "../../utils";
    import {Browser} from "@wailsio/runtime"

    let {
        contextTriggerArea,
        addon,
    }: {
        contextTriggerArea: Snippet;
        addon: Addon;
    } = $props();

    async function handleUninstall() {
        console.debug("Uninstalling addon: ", addon.name);
        let didUninstall = await addons.uninstall(addon.name);
        if (didUninstall) {
            console.debug("Uninstalled addon: ", addon.name);
            toast.success(`${addon.alias} was uninstalled`);
        } else {
            console.error("Failed to uninstall addon: ", addon.name);
            toast.error(`Failed to uninstall ${addon.alias}`);
        }
    }

    async function handleUnmanage() {
        console.debug("Unmanaging addon: ", addon.name);
        if (await addons.unmanage(addon.name)) {
            console.debug("Unmanaged addon: ", addon.name);
            toast.success(`${addon.alias} was unmanaged`);
        } else {
            console.error("Failed to unmanage addon: ", addon.name);
            toast.error(`Failed to unmanage ${addon.alias}`);
        }
    }

    async function handleOpenDirectory() {
        try {
            await LocalAddonService.OpenDirectory(addon.name);
        } catch (e) {
            console.error("Failed to open directory: ", e);
            toast.error(`Failed to open directory: ${e}`);
        }
    }
</script>

<ContextMenu.Root>
    <ContextMenu.Trigger>
        {@render contextTriggerArea()}
    </ContextMenu.Trigger>
    <ContextMenu.Content>
        <ContextMenu.Item class="gap-2" onclick={handleOpenDirectory}>
            <FolderOpen size={16}/>
            Open directory
        </ContextMenu.Item>
        {#if addon.isManaged}
            <ContextMenu.Item
                    class="gap-3"
                    onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}
            >
                <Github size={16}/>
                View code
            </ContextMenu.Item>
            <ContextMenu.Item
                    class="gap-2"
                    onclick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
            >
                <Bug size={16}/>
                Report issue
            </ContextMenu.Item>
        {/if}

        <ContextMenu.Item class="gap-2" onclick={handleUninstall}>
            <Trash size={16}/>
            Uninstall
        </ContextMenu.Item>

        {#if addon.isManaged}
            <ContextMenu.Item class="gap-2" onclick={handleUnmanage}>
                <AlertTriangle size={16}/>
                Unmanage
            </ContextMenu.Item>
        {/if}
    </ContextMenu.Content>
</ContextMenu.Root>
