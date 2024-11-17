<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import {Button} from "$lib/components/ui/button/index.js";
    import {addon as ad} from "../../../wailsjs/go/models";
    import addons from "../../addons";
    import {toast} from "svelte-sonner";

    let {
        open = $bindable(),
        onOpenChange,
        addon,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: ad.Addon;
    } = $props();

    async function handleMatchAddon() {
        let manifest = await addons.getManifest(addon.name)
        if (!manifest) {
            console.error('Failed to get manifest, this should not happen...')
            return;
        }

        try {
            await addons.install(manifest)
        } catch (e: any) {
            if (e.toString().includes('no release found')) {
                // TODO: Show error message
                // dialogErrorMsg = "No release found for this addon.";
            }
            return;
        }

        toast.success('Addon matched', {
            description: `${addon.displayName} was matched and will be managed by Classic Addon Manager`,
            duration: 7000
        });
        open = false;
    }
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title>{addon.displayName}</Dialog.Title>
            <Dialog.Description>
                {addon.description}
            </Dialog.Description>
        </Dialog.Header>
        <div class="flex flex-col">
            {#if !addon.isManaged}
                <p>This addon is not managed by Classic Addon Manager.</p>

                {#await addons.repoHasAddon(addon.name)}
                    <span></span>
                {:then found}
                    {#if found}
                        <h3 class="pt-4 pb-1 font-semibold">However!</h3>
                        <p>The addon was found in our addon repository.</p>
                        <p>Do you want Classic Addon Manager to manage updates?</p>
                        <Button class="mt-4" onclick={handleMatchAddon}>
                            Yes, do it!
                        </Button>
                    {/if}
                {/await}
            {/if}
        </div>
    </Dialog.Content>
</Dialog.Root>