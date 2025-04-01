<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import Like from "lucide-svelte/icons/thumbs-up";
    import Dislike from "lucide-svelte/icons/thumbs-down";

    import {Button} from "$lib/components/ui/button/index";
    import type {Addon, Release} from "$lib/wails";
    import addons from "../../addons";
    import {isAuthenticated} from "$stores/UserStore.svelte";
    import {apiClient} from "../../api";
    import {toast} from "../../utils";
    import {RemoteAddonService} from "$lib/wails";
    import LocalAddonUpdateDialog from "./LocalAddonUpdateDialog.svelte";

    let {
        open = $bindable(),
        onOpenChange,
        addon,
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: Addon;
    } = $props();

    let uninstallClicks = $state(0);
    let uninstallTimeout: any;
    let rating = $state(0);
    let latestRelease: Release | undefined = $state();
    let openUpdateDialog = $state(false);

    function handleOpenUpdateDialogChange(o: boolean) {
        openUpdateDialog = o;
    }

    $effect(() => {
        getMyRating(open);
        if (open && addon.isManaged) {
            checkForUpdates();
        }
    });

    async function checkForUpdates() {
        try {
            latestRelease = await RemoteAddonService.GetLatestRelease(addon.name);
        } catch (e) {
            console.error("Failed to get release for addon: ", addon.name);
        }
    }

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
            {description: `${addon.alias} was reinstalled`, duration: 7000}
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

{#if latestRelease}
    <LocalAddonUpdateDialog
        {addon}
        release={latestRelease}
        bind:open={openUpdateDialog}
        onOpenChange={handleOpenUpdateDialogChange}
    />
{/if}

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content class="max-w-2xl">
        <Dialog.Header class="pb-6">
            <div class="flex items-start justify-between">
                <div class="space-y-1.5">
                    <Dialog.Title class="text-2xl font-semibold tracking-tight">
                        {addon.alias}
                    </Dialog.Title>
                    <div class="flex items-center gap-3 text-sm">
                        {#if addon.author}
                            <div class="flex items-center gap-2 px-2 py-0.5 bg-muted/50 rounded-full">
                                <span class="text-xs text-muted-foreground">by</span>
                                <span class="font-medium">{addon.author}</span>
                            </div>
                        {/if}
                        {#if addon.isManaged}
                            <div class="flex items-center gap-1.5">
                                <div class="w-2 h-2 rounded-full {latestRelease && latestRelease.published_at > addon.updatedAt ? 'bg-amber-500' : 'bg-emerald-500'}"></div>
                                <span class="font-medium {latestRelease && latestRelease.published_at > addon.updatedAt ? 'text-amber-500' : 'text-emerald-500'}">{addon.version}</span>
                                {#if latestRelease && latestRelease.published_at > addon.updatedAt}
                                    <div class="flex items-center gap-2">
                                        <Button 
                                            size="sm" 
                                            variant="ghost" 
                                            class="h-6 px-2 text-xs hover:text-amber-500 hover:bg-amber-500/10"
                                            onclick={() => {
                                                open = false;
                                                openUpdateDialog = true;
                                            }}
                                        >
                                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-1"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 21h5v-5"/></svg>
                                            Update
                                        </Button>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                </div>

                {#if isAuthenticated()}
                    <div class="flex gap-1.5 mr-8">
                        <Button 
                            size="sm"
                            class="relative {rating === 1 ? 'bg-blue-100 dark:bg-blue-900/30 border-blue-500' : ''}"
                            variant="outline" 
                            onclick={() => handleRating(1)}
                        >
                            <div class="absolute inset-0 bg-blue-500/10 scale-0 group-hover:scale-100 transition-transform rounded-md"></div>
                            {#if rating === 1}
                                <Like class="w-4 h-4 text-blue-500"/>
                            {:else}
                                <Like class="w-4 h-4 text-muted-foreground hover:text-blue-500 transition-colors"/>
                            {/if}
                        </Button>
                        <Button 
                            size="sm"
                            class="relative {rating === -1 ? 'bg-red-100 dark:bg-red-900/30 border-red-500' : ''}"
                            variant="outline" 
                            onclick={() => handleRating(-1)}
                        >
                            <div class="absolute inset-0 bg-red-500/10 scale-0 group-hover:scale-100 transition-transform rounded-md"></div>
                            {#if rating === -1}
                                <Dislike class="w-4 h-4 text-red-500"/>
                            {:else}
                                <Dislike class="w-4 h-4 text-muted-foreground hover:text-red-500 transition-colors"/>
                            {/if}
                        </Button>
                    </div>
                {/if}
            </div>
        </Dialog.Header>

        <div class="space-y-6">
            <div class="prose prose-sm dark:prose-invert">
                <Dialog.Description class="leading-relaxed">
                    {addon.description}
                </Dialog.Description>
            </div>

            {#if !addon.isManaged}
                <div class="p-4 rounded-lg border bg-card">
                    <div class="space-y-3">
                        <div class="flex items-center gap-2 text-muted-foreground">
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-info"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
                            <p class="text-sm font-medium">This addon is not managed by Classic Addon Manager</p>
                        </div>

                        {#await addons.repoHasAddon(addon.name)}
                            <div class="flex items-center gap-2 text-sm text-muted-foreground">
                                <div class="w-3 h-3 rounded-full border-2 border-t-transparent border-current animate-spin"></div>
                                <span>Checking repository...</span>
                            </div>
                        {:then found}
                            {#if found}
                                <div class="space-y-3">
                                    <p class="text-sm">Good news! This addon was found in our repository. Would you like Classic Addon Manager to handle updates for you?</p>
                                    <Button class="w-full sm:w-auto" onclick={handleMatchAddon}>
                                        Yes, manage this addon
                                    </Button>
                                </div>
                            {/if}
                        {/await}
                    </div>
                </div>
            {/if}
        </div>

        {#if addon.isManaged}
            <Dialog.Footer class="mt-6">
                <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2 w-full">
                    <Button 
                        type="button" 
                        variant="outline" 
                        class="flex-1 sm:flex-initial" 
                        onclick={handleReinstall}
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 21h5v-5"/></svg>
                        Reinstall
                    </Button>
                    <Button 
                        type="button" 
                        variant="destructive" 
                        class="flex-1 sm:flex-initial"
                        onclick={handleUninstall}
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                        {uninstallClicks === 1 ? 'Click to confirm' : 'Uninstall'}
                    </Button>
                </div>
            </Dialog.Footer>
        {/if}
    </Dialog.Content>
</Dialog.Root>