<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index";
    import {Button} from "$lib/components/ui/button/index";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";
    import {Tag, User, CalendarDays, Package, ArrowUpCircle} from "lucide-svelte";
    import {Badge} from "$lib/components/ui/badge/index";
    import addons from "../../addons";
    import {getUpdatesAvailableCount, setUpdatesAvailableCount} from "$stores/AddonStore.svelte";
    import {formatToLocalTime, toast, safeCall} from "../../utils";
    import type {Addon, Release} from "$lib/wails";
    import {marked} from "marked";
    import DOMPurify from "dompurify";

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
    let changelog = $state("");

    marked.setOptions({
        gfm: true,
        breaks: true,
    });

    $effect(() => {
        if (open && release?.body) {
            changelog = DOMPurify.sanitize(marked.parse(release.body, {async: false}));
        } else {
            changelog = "No change log was provided by the addon";
        }
    });

    async function handleUpdateClick() {
        isUpdating = true;

        const updateOperation = async () => {
            const manifest = await addons.getManifest(addon.name);
            if (!manifest) throw new Error('Manifest not found');
            return await addons.install(manifest, 'latest');
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
    <Dialog.Content class="sm:max-w-[600px] max-h-[90svh] flex flex-col p-0">
        <Dialog.Header class="p-6 pb-4 shrink-0 border-b">
            <Dialog.Title class="text-2xl font-semibold mb-1">{addon.alias}</Dialog.Title>
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
                <span class="inline-flex items-center gap-1">
                    <User class="w-3.5 h-3.5"/>
                    <span class="font-normal text-foreground/90">{addon.author}</span>
                </span>
                {#if release}
                    <span class="inline-flex items-center gap-x-1.5">
                        <Badge variant="secondary" class="inline-flex items-center gap-1">
                            <Tag class="w-3 h-3"/>
                            {release.tag_name}
                        </Badge>
                        <span class="inline-flex items-center gap-1 text-xs text-muted-foreground">
                            <CalendarDays class="w-3 h-3"/>
                            {formatToLocalTime(release.published_at)}
                        </span>
                    </span>
                {:else}
                    <Badge variant="outline">No Release</Badge>
                {/if}
            </div>
            {#if addon.description}
                <Dialog.Description class="text-sm text-muted-foreground">{addon.description}</Dialog.Description>
            {/if}
        </Dialog.Header>

        <div class="flex-1 overflow-y-auto p-6">
            {#if release}
                <div class="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4">
                    <CalendarDays class="w-3.5 h-3.5"/>
                    <span>Released on {formatToLocalTime(release.published_at)}</span>
                </div>
                <div class="border rounded-lg p-4 bg-card">
                    <div class="prose max-w-none text-sm text-foreground dark:text-foreground/90">
                        {@html changelog}
                    </div>
                </div>
            {:else}
                <div class="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
                    <Package class="w-12 h-12 mb-4 opacity-50"/>
                    <p class="font-medium">No changelog available</p>
                    <p class="text-xs mt-1">The author hasn't provided release notes.</p>
                </div>
            {/if}
        </div>

        <Dialog.Footer class="p-4 border-t">
            {#if addon.isManaged}
                {#if isUpdating}
                    <Button type="button" variant="default" disabled class="w-full sm:w-auto">
                        <LoaderCircle class="w-4 h-4 mr-2 animate-spin"/>
                        Updating...
                    </Button>
                {:else}
                    <Button
                            type="button"
                            variant="default"
                            onclick={handleUpdateClick}
                            class="w-full sm:w-auto"
                    >
                        <ArrowUpCircle class="w-4 h-4 mr-2"/>
                        Update to {release.tag_name}
                    </Button>
                {/if}
            {/if}
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>