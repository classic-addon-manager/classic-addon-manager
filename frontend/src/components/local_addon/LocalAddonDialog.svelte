<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import Like from "lucide-svelte/icons/thumbs-up";
    import Dislike from "lucide-svelte/icons/thumbs-down";
    import {User, Tag, Package, ArrowUpCircle, RefreshCcw, Trash2} from "lucide-svelte";
    import {Badge} from "$lib/components/ui/badge/index";

    import {Button} from "$lib/components/ui/button/index";
    import type {Addon, Release} from "$lib/wails";
    import addons from "../../addons";
    import {isAuthenticated} from "$stores/UserStore.svelte";
    import {apiClient} from "../../api";
    import {toast, safeCall, formatToLocalTime} from "../../utils";
    import {AddonManifest, RemoteAddonService} from "$lib/wails";
    import LocalAddonUpdateDialog from "./LocalAddonUpdateDialog.svelte";
    import {marked} from "marked";
    import DOMPurify from "dompurify";
    import RemoteAddonReadme from "../remote_addon/RemoteAddonReadme.svelte";

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
    let readme = $state("Loading description...");

    marked.setOptions({
        gfm: true,
        breaks: true,
    });

    $effect(() => {
        if (open) {
            getMyRating();
            if (addon.isManaged) {
                checkForUpdates();
            }
            getReadme();
        }
    });

    async function getReadme() {
        try {
            if (!addon.repo) {
                const fallbackReadme = addon.description || "No description provided.";
                readme = DOMPurify.sanitize(marked.parse(fallbackReadme, {async: false}));
                return;
            }

            // Default to 'main' branch if not specified
            const branch = 'main';
            const response = await fetch(
                `https://raw.githubusercontent.com/${addon.repo}/${branch}/README.md`,
            );
            if (!response.ok) {
                const fallbackReadme = addon.description || "No description provided.";
                readme = DOMPurify.sanitize(marked.parse(fallbackReadme, {async: false}));
                return;
            }
            const rawReadme = await response.text();
            readme = DOMPurify.sanitize(await marked.parse(rawReadme));
        } catch (e) {
            console.error("Error fetching README:", e);
            const errorReadme = addon.description || "Error loading description.";
            readme = DOMPurify.sanitize(marked.parse(errorReadme, {async: false}));
        }
    }

    function handleOpenUpdateDialogChange(o: boolean) {
        openUpdateDialog = o;
    }

    async function checkForUpdates() {
        const [release, error] = await safeCall<Release>(() => RemoteAddonService.GetLatestRelease(addon.name));
        if (error) {
            console.error("Failed to get release for addon: ", addon.name, error);
            return;
        }
        if (release) {
            latestRelease = release;
        }
    }

    async function handleMatchAddon() {
        const [manifest, manifestError] = await safeCall<AddonManifest>(() => addons.getManifest(addon.name));
        if (manifestError || !manifest) {
            console.error('Failed to get manifest:', manifestError);
            toast.error(`Failed to get manifest: ${manifestError || 'Unknown error'}`);
            return;
        }

        const [installError] = await safeCall<boolean>(() => addons.install(manifest, 'latest'));
        if (installError) {
            const errorString = String(installError);
            if (errorString.includes('no release found')) {
                toast.error("No release found for this addon.");
            } else {
                console.error("Failed to install addon during match:", installError);
                toast.error(`Failed to match addon: ${errorString.substring(0, 100)}`);
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
        const [manifest, manifestError] = await safeCall<AddonManifest>(() => addons.getManifest(addon.name));

        if (manifestError || !manifest) {
            if (manifestError.toString().includes('no release found')) {
                toast.error('No release found for this addon');
            } else {
                toast.error('Failed to reinstall addon');
            }
            return;
        }

        let didInstall: boolean = await addons.install(manifest, 'latest')
        if (!didInstall) return;
        toast.success('Addon reinstalled',
            {description: `${addon.alias} was reinstalled`, duration: 7000}
        );
        open = false;
    }

    function getMyRating() {
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
    <Dialog.Content class="sm:max-w-[600px] max-h-[90svh] flex flex-col p-0">
        <Dialog.Header class="p-6 pb-4 shrink-0 border-b">
            <Dialog.Title class="text-2xl font-semibold mb-1">{addon.alias}</Dialog.Title>
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
                <span class="inline-flex items-center gap-1">
                    <User class="w-3.5 h-3.5"/>
                    <span class="font-normal text-foreground/90">{addon.author}</span>
                </span>
                {#if addon.version}
                    <Badge variant="secondary" class="inline-flex items-center gap-1">
                        <Tag class="w-3 h-3"/>
                        {addon.version}
                    </Badge>
                {:else}
                    <Badge variant="outline">No Version</Badge>
                {/if}
            </div>
            {#if addon.description}
                <Dialog.Description class="text-sm text-muted-foreground">{addon.description}</Dialog.Description>
            {/if}
        </Dialog.Header>

        <div class="flex-1 overflow-y-auto p-6">
            {#if readme === "Loading description..."}
                <div class="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
                    <Package class="w-12 h-12 mb-4 opacity-50"/>
                    <p class="font-medium">Loading description...</p>
                </div>
            {:else}
                <RemoteAddonReadme {readme}/>
            {/if}

            {#if !addon.isManaged}
                <div class="border rounded-lg p-4 bg-card mt-6">
                    <div class="space-y-3">
                        <div class="flex items-center gap-2 text-muted-foreground">
                            <Package class="w-4 h-4"/>
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
                                    <p class="text-sm text-muted-foreground">Good news! This addon was found in our
                                        repository. Would you like Classic Addon Manager to handle updates for you?</p>
                                    <Button class="w-full sm:w-auto" onclick={handleMatchAddon}>
                                        <ArrowUpCircle class="w-4 h-4 mr-2"/>
                                        Yes, manage this addon
                                    </Button>
                                </div>
                            {/if}
                        {/await}
                    </div>
                </div>
            {/if}
        </div>

        <Dialog.Footer class="p-4 border-t">
            <div class="flex justify-between items-center w-full gap-4">
                {#if addon.isManaged}
                    <div class="flex gap-1 items-center">
                        {#if isAuthenticated()}
                            <Button
                                    variant="ghost"
                                    size="icon"
                                    class="h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-blue-100 dark:hover:bg-blue-900/30 {rating === 1 ? 'bg-blue-100 dark:bg-blue-900/30 border border-blue-500' : ''}"
                                    onclick={() => handleRating(1)}
                                    aria-label="Like addon"
                            >
                                <Like class="w-5 h-5 {rating === 1 ? 'text-blue-500' : 'text-muted-foreground group-hover:text-blue-500'}"/>
                            </Button>
                            <Button
                                    variant="ghost"
                                    size="icon"
                                    class="h-8 w-9 transition-all duration-200 hover:scale-105 hover:bg-red-100 dark:hover:bg-red-900/30 {rating === -1 ? 'bg-red-100 dark:bg-red-900/30 border border-red-500' : ''}"
                                    onclick={() => handleRating(-1)}
                                    aria-label="Dislike addon"
                            >
                                <Dislike
                                        class="w-5 h-5 {rating === -1 ? 'text-red-500' : 'text-muted-foreground group-hover:text-red-500'}"/>
                            </Button>
                        {:else}
                            <span class="flex items-center gap-1 p-2 cursor-not-allowed opacity-50"
                                  aria-label="Log in to rate addons">
                                <Like class="w-5 h-5 text-muted-foreground"/>
                                <Dislike class="w-5 h-5 text-muted-foreground"/>
                            </span>
                        {/if}
                    </div>

                    <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
                        <Button
                                type="button"
                                variant="outline"
                                onclick={handleReinstall}
                        >
                            <RefreshCcw class="w-4 h-4 mr-2"/>
                            Reinstall
                        </Button>
                        <Button
                                type="button"
                                variant="destructive"
                                onclick={handleUninstall}
                        >
                            <Trash2 class="w-4 h-4 mr-2"/>
                            {uninstallClicks === 1 ? 'Click to confirm' : 'Uninstall'}
                        </Button>
                    </div>
                {/if}
            </div>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>