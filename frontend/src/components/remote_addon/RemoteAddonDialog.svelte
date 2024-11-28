<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import {addon as ad, api} from "../../../wailsjs/go/models";
    import {onMount} from "svelte";
    import addons from "../../addons.js";
    import {GetLatestRelease as GoGetLatestRelease, IsAddonInstalled} from "../../../wailsjs/go/main/App";
    import {toast} from "svelte-sonner";
    import {Button} from "$lib/components/ui/button";

    let {
        open = $bindable(),
        onOpenChange,
        addon
    }: {
        open: boolean;
        onOpenChange: (open: boolean) => void;
        addon: ad.AddonManifest;
    } = $props();

    let release: api.Release | undefined = $state();
    let isInstalled = $state(false);
    let hasBanner = $state(false);
    let banner: string = $state('');

    onMount(async () => {
        isInstalled = await IsAddonInstalled(addon.name);
        hasBanner = await checkForBanner();
    });

    async function checkForBanner() {
        const response = await fetch(`https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/banner.png`);
        if (!response.ok) {
            return false;
        }

        const reader = response.body?.getReader();
        if (reader) {
            const chunks = [];
            let done = false;
            while (!done) {
                let result;
                try {
                    result = await reader.read();
                } catch (e) {
                    console.error("Error reading image:", e);
                    return false;
                }
                done = result.done;
                if (result.value) {
                    chunks.push(result.value);
                }
            }
            const blob = new Blob(chunks);
            banner = URL.createObjectURL(blob);
        }
        return true;
    }

    async function getRelease(open: boolean) {
        if (!open) return;
        try {
            release = await GoGetLatestRelease(addon.name);
        } catch (e) {
            toast.error(`Failed to fetch release information for ${addon.name}`);
            console.error(e);
        }
    }

    // Whenever the dialog is opened, fetch the latest release information
    $effect(() => {
        getRelease(open);
    });

    function formatToLocalTime(dateString: string): string {
        const date = new Date(dateString);
        const options: Intl.DateTimeFormatOptions = {
            hour: '2-digit',
            minute: '2-digit',
            day: '2-digit',
            month: 'long',
            year: 'numeric'
        };
        return date.toLocaleString(undefined, options).replace(',', '');
    }

    async function handleInstall(): Promise<void> {
        if (isInstalled) {
            toast.error('Addon is already installed');
            return;
        }
        let didInstall: boolean = false;
        try {
            didInstall = await addons.install(addon);
            if (!didInstall) {
                toast.error('Failed to install addon');
            }
        } catch (e) {
            if (e == 'no release found') {
                toast.error(`No release found for ${addon.name}`);
            } else {
                toast.error('Failed to install addon');
            }
        }

        if (didInstall) {
            toast.success('Addon installed', {description: `${addons.nameToDisplayName(addon.name)} was installed`});
            isInstalled = true;
            open = false;
        }
    }
</script>

<Dialog.Root {open} {onOpenChange}>
    <Dialog.Content class="max-h-[90%]">
        <Dialog.Header>
            <Dialog.Title>
                {addons.nameToDisplayName(addon.name)} <span
                    class="text-muted-foreground">{release ? release.tag_name : ''}</span>
            </Dialog.Title>
        </Dialog.Header>
        <Tabs.Root value="description" class="w-full">
            <Tabs.List class="grid w-full grid-cols-2">
                <Tabs.Trigger value="description">Description</Tabs.Trigger>
                <Tabs.Trigger value="changelog">Changelog</Tabs.Trigger>
            </Tabs.List>
            <Tabs.Content value="description">
                <div class="flex flex-1 flex-col max-h-[calc(100vh-30vh)] overflow-auto">
                    {#if hasBanner}
                        <img class="max-h-[40svh] aspect-video object-fill" src={banner} alt=""/>
                    {/if}
                    <div>
                        <p class="mt-4">{addon.description}</p>
                    </div>
                </div>
            </Tabs.Content>
            <Tabs.Content value="changelog">
                <p class="text-muted-foreground">Released {formatToLocalTime(release?.published_at)}</p>
                <p class="whitespace-pre-wrap">{release?.body || 'No change log was provided by the addon'}</p>
            </Tabs.Content>
        </Tabs.Root>
        <div class="flex">
            {#if !isInstalled}
                <Button
                        class="ml-auto mt-2"
                        variant="default"
                        onclick={handleInstall}
                >
                    Install
                </Button>
            {/if}
        </div>
    </Dialog.Content>
</Dialog.Root>