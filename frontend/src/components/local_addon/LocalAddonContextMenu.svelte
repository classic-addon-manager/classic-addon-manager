<script lang="ts">
    import type {Snippet} from "svelte";
    import {addon as ad} from "../../../wailsjs/go/models";
    import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
    import Trash from "lucide-svelte/icons/trash";
    import Bug from "lucide-svelte/icons/bug";
    import Github from "lucide-svelte/icons/github";
    import {toast} from "svelte-sonner";
    import addons from "../../addons";

    let {
        contextTriggerArea,
        addon,
    }: {
        contextTriggerArea: Snippet;
        addon: ad.Addon;
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
</script>

<ContextMenu.Root>
    <ContextMenu.Trigger>
        {@render contextTriggerArea()}
    </ContextMenu.Trigger>
    <ContextMenu.Content>
        {#if addon.isManaged}
            <ContextMenu.Item
                    class="gap-3"
                    onclick={() => {
                    // @ts-ignore
                    window.runtime.BrowserOpenURL(
                        `https://github.com/${addon.repo}`,
                    );
                }}
            >
                <Github size={16}/>
                View code
            </ContextMenu.Item>
            <ContextMenu.Item
                    class="gap-2"
                    onclick={() => {
                    // @ts-ignore
                    window.runtime.BrowserOpenURL(
                        `https://github.com/${addon.repo}/issues/new`,
                    );
                }}
            >
                <Bug size={16}/>
                Report issue
            </ContextMenu.Item>
        {/if}

        <ContextMenu.Item class="gap-2" onclick={handleUninstall}>
            <Trash size={16}/>
            Uninstall
        </ContextMenu.Item>
    </ContextMenu.Content>
</ContextMenu.Root>
