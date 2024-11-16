<script lang="ts">
    import type { Snippet } from "svelte";
    import { addon } from "../../../wailsjs/go/models";
    import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
    import Trash from "lucide-svelte/icons/trash";
    import Bug from "lucide-svelte/icons/bug";
    import Github from "lucide-svelte/icons/github";

    let {
        content,
        addonData,
    }: {
        content: Snippet;
        addonData: addon.Addon;
    } = $props();
</script>

<ContextMenu.Root>
    <ContextMenu.Trigger>
        {@render content()}
    </ContextMenu.Trigger>
    <ContextMenu.Content>
        {#if addonData.isManaged}
            <ContextMenu.Item
                class="gap-3"
                onclick={() => {
                    // @ts-ignore
                    window.runtime.BrowserOpenURL(
                        `https://github.com/${addonData.repo}`,
                    );
                }}
            >
                <Github size={16} /> View code
            </ContextMenu.Item>
            <ContextMenu.Item
                class="gap-2"
                onclick={() => {
                    // @ts-ignore
                    window.runtime.BrowserOpenURL(
                        `https://github.com/${addonData.repo}/issues/new`,
                    );
                }}
            >
                <Bug size={16} /> Report issue
            </ContextMenu.Item>
        {/if}

        <ContextMenu.Item class="gap-2">
            <Trash size={16} /> Uninstall
        </ContextMenu.Item>
    </ContextMenu.Content>
</ContextMenu.Root>
