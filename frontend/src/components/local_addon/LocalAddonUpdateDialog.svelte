<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import {Button} from "$lib/components/ui/button/index";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import addons from "../../addons";
    import {getUpdatesAvailableCount, setUpdatesAvailableCount} from "$stores/AddonStore.svelte";
    import {formatToLocalTime, toast, safeCall} from "../../utils";
    import type {Addon, Release} from "$lib/wails";

    let {
        open = $bindable(),
        onOpenChange,
        addon,
        release
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: Addon;
        release: Release;
    } = $props();

    let isUpdating = $state(false);

    async function handleUpdateClick() {
        isUpdating = true;
        
        const updateOperation = async () => {
            const manifest = await addons.getManifest(addon.name);
            if (!manifest) throw new Error('Manifest not found');
            return await addons.install(manifest);
        };

        const [didInstall, error] = await safeCall(updateOperation);

        if (error) {
            const errorString = String(error); 
            if (errorString.includes('no release found')) {
                toast.error('No release found for this addon');
            } else {
                console.error("Failed to update addon:", error);
                toast.error(`Failed to update addon: ${errorString.substring(0, 100)}`);
            }
            isUpdating = false;
            return;
        }
        
        isUpdating = false; 
        if (!didInstall) return;
        
        toast.success('Addon updated',
            {
                description: `${addon.alias} was updated to ${release.tag_name}`,
                duration: 7000
            }
        );
        setUpdatesAvailableCount(getUpdatesAvailableCount() - 1);
        open = false;
    }
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title>
                {addon.alias}
                {#if addon.isManaged}
                    <span class="text-muted-foreground">{release.tag_name}</span>
                {/if}
            </Dialog.Title>
            <Dialog.Description>{addon.description}</Dialog.Description>
        </Dialog.Header>
        <div class="flex flex-col">
            <p class="text-muted-foreground">Released {formatToLocalTime(release.published_at)}</p>
            <p>{release.body || 'No change log was provided by the addon'}</p>
        </div>
        {#if addon.isManaged}
            <Dialog.Footer>
                {#if isUpdating}
                    <Button type="button" variant="default" disabled>
                        <LoaderCircle class="animate-spin" size={16}/>
                        Updating...
                    </Button>
                {:else}
                    <Button type="button" variant="default" onclick={handleUpdateClick}>Update</Button>
                {/if}
            </Dialog.Footer>
        {/if}
    </Dialog.Content>
</Dialog.Root>