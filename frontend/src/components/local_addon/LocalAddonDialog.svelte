<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import Like from "lucide-svelte/icons/thumbs-up";
    import Dislike from "lucide-svelte/icons/thumbs-down";

    import {Button} from "$lib/components/ui/button/index";
    import {addon as ad} from "../../../wailsjs/go/models";
    import addons from "../../addons";
    import {isAuthenticated} from "$stores/UserStore.svelte";
    import {apiClient} from "../../api";
    import { toast } from "../../utils";

    let {
        open = $bindable(),
        onOpenChange,
        addon,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: ad.Addon;
    } = $props();

    let uninstallClicks = $state(0);
    let uninstallTimeout: any;
    let rating = $state(0);

    $effect(() => {
        getMyRating(open);
    });

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

        toast.success('Addon matched',
            {
                description: `${addon.alias} was matched and will be managed by Classic Addon Manager`,
                duration: 7000
            }
        );
        open = false;
    }

    async function handleUninstall() {
        if (uninstallClicks === 0) {
            toast.info('Click again to uninstall', {dismissable: true});
        }
        uninstallClicks++;

        uninstallTimeout = setTimeout(() => {
            uninstallClicks = 0;
        }, 5000);

        if (uninstallClicks === 2) {
            clearTimeout(uninstallTimeout);
            uninstallClicks = 0;
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
    }

    async function handleReinstall() {
        let didInstall = false;
        try {
            didInstall = await addons.install(await addons.getManifest(addon.name))
        } catch (e: any) {
            if (e.toString().includes('no release found')) {
                toast.error('No release found for this addon');
            } else {
                toast.error('Failed to update addon');
            }
            return;
        }
        if (!didInstall) return;
        toast.success('Addon reinstalled',
            { description: `${addon.alias} was reinstalled`, duration: 7000 }
        );
        open = false;
    }

    function getMyRating(open: boolean) {
        if (!open || !isAuthenticated()) return;
        apiClient.get(`/addon/${addon.name}/my-rating`).then(async rateResponse => {
            if (rateResponse.status === 200) {
                const r = await rateResponse.json();
                rating = r.data.rating;
            }
        }).catch(e => {
            console.error('Failed to fetch rating', e);
        });
    }

    async function handleRating(r: number) {
        if (rating === r || r === 0) {
            return;
        }
        const response = await apiClient.post(`/addon/${addon.name}/rate`, {
            is_like: r === 1
        });

        if (response.status !== 200) {
            toast.error('Could not rate addon, try again later');
            return;
        }

        rating = r;

        if (r === 1) {
            toast.success('Addon rated', {description: `Liked ${addon.alias}`});
        } else {
            toast.success('Addon rated', {description: `Disliked ${addon.alias}`});
        }
    }
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title>
                {addon.alias}
                {#if addon.isManaged}
                    <span class="text-muted-foreground">{addon.version}</span>
                {/if}
            </Dialog.Title>
            <Dialog.Description>{addon.description}</Dialog.Description>
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
        {#if addon.isManaged}
            <Dialog.Footer>
                {#if isAuthenticated() }
                    <div class="flex gap-2 items-center mr-auto">
                        <Button class="mt-2" variant="outline" onclick={() => handleRating(1)}>
                            {#if rating === 1}
                                <Like class="w-6 text-blue-500"/>
                            {:else}
                                <Like class="w-6"/>
                            {/if}
                        </Button>
                        <Button class="mt-2" variant="outline" onclick={() => handleRating(-1)}>
                            {#if rating === -1}
                                <Dislike class="w-6 text-red-500"/>
                            {:else}
                                <Dislike class="w-6"/>
                            {/if}
                        </Button>
                    </div>
                {/if}

                <div class="flex gap-2 items-end">
                    <Button type="button" variant="outline" onclick={handleReinstall}>Reinstall</Button>
                    <Button type="button" variant="destructive" onclick={handleUninstall}>Uninstall</Button>
                </div>
            </Dialog.Footer>
        {/if}
    </Dialog.Content>
</Dialog.Root>