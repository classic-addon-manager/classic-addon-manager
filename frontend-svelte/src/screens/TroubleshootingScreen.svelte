<script lang="ts">
    import * as Card from "$lib/components/ui/card/index";
    import * as Accordion from "$lib/components/ui/accordion/index";
    import * as Collapsible from "$lib/components/ui/collapsible/index";
    import {ScrollArea} from "$lib/components/ui/scroll-area/index";
    import {Badge} from "$lib/components/ui/badge/index";
    import {Button} from "$lib/components/ui/button/index";
    import type {Addon, LogParseResult} from "$lib/wails";
    import {LocalAddonService} from "$lib/wails";
    import {getInstalledAddons} from "$stores/AddonStore.svelte";
    import addons from "../addons";
    import {onMount} from "svelte";
    import {ChevronsUpDown} from "lucide-svelte";
    import {toast} from "../utils";
    import {slide} from "svelte/transition";
    import {cubicOut} from "svelte/easing";
    import {safeCall} from "../utils";

    let issues: LogParseResult[] = $state([]);
    let issueCount = $derived.by(() => issues.length);

    let groupedIssues = $derived.by(() => {
        let grouped: { [key: string]: { type: string, error: string, file: string }[] } = {};
        for (let issue of issues) {
            if (!grouped[issue.Addon]) {
                grouped[issue.Addon] = [];
            }
            grouped[issue.Addon].push({
                type: issue.Type,
                error: issue.Error,
                file: issue.File
            });
        }
        return grouped;
    });

    onMount(async () => {
        issues = await LocalAddonService.DiagnoseIssues();
    });

    async function handleResetAddonSettings() {
        const [, error] = await safeCall(LocalAddonService.ResetSettings);
        if (error) {
            console.error("Failed to reset addon settings", error);
            toast.error(`Failed to reset addon settings, reason: ${error}`, {
                duration: 6000,
            });
            return;
        }
        toast.error("Addon settings have been reset, please restart the game");
    }

    async function handleUninstallAllAddons() {
        let ads: Addon[] = getInstalledAddons();
        if (ads.length === 0) {
            toast.error("No addons to uninstall");
            return;
        }
        let uninstallCount = 0;
        for (let a of ads) {
            let uninstalled = await LocalAddonService.UninstallAddon(a.name);
            if (!uninstalled) {
                toast.error(`Failed to uninstall addon ${a.alias}`);
                return;
            }
            uninstallCount++;
        }
        toast.success(`Uninstalled ${uninstallCount} addons`);
        await addons.populateAddonStore();
        await handleResetAddonSettings();
    }
</script>

<div class="flex flex-col h-screen">
    <header class="bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
        <div class="container flex h-16 items-center justify-center gap-4 px-4">
            <div class="flex flex-col items-center gap-1">
                <h1 class="text-xl font-semibold">Troubleshooting</h1>
                <p class="text-sm text-muted-foreground">Select an option below to help resolve your issue</p>
            </div>
        </div>
    </header>

    <main class="flex-1 overflow-auto">
        <div class="container px-4 py-6">
            <div class="mx-auto max-w-3xl">
                <Accordion.Root type="single" class="space-y-4">
                    {#each [{id: 'item-1', title: 'Specific Addon Issue'}, {id: 'item-2', title: 'No Addons Are Working'}, {id: 'item-3', title: 'Diagnostic Results'}] as item}
                        <Accordion.Item value={item.id} class="border rounded-lg overflow-hidden">
                            <Accordion.Trigger class="hover:bg-muted/50 transition-colors px-6 w-full [&[data-state=open]]:no-underline hover:no-underline no-underline">
                                <div class="flex items-center gap-2 w-full">
                                    <span class="text-lg">{item.title}</span>
                                    {#if item.id === 'item-3'}
                                        {#if issueCount > 0}
                                            <Badge variant="destructive" class="ml-auto mr-4">
                                                {issueCount} {issueCount === 1 ? 'issue' : 'issues'} found
                                            </Badge>
                                        {:else}
                                            <Badge variant="successful" class="ml-auto">
                                                No issues detected
                                            </Badge>
                                        {/if}
                                    {/if}
                                </div>
                            </Accordion.Trigger>
                            <Accordion.Content>
                                {#if item.id === 'item-1'}
                                    <div transition:slide={{duration: 200, easing: cubicOut}} class="px-6 py-4 bg-muted/20">
                                        <div class="space-y-3">
                                            <p class="text-muted-foreground">If a specific addon is not working as expected:</p>
                                            <ol class="list-decimal list-inside space-y-2 text-muted-foreground">
                                                <li>Go to the Dashboard</li>
                                                <li>Right-click the problematic addon</li>
                                                <li>Select "Report issue"</li>
                                            </ol>
                                            <div class="bg-muted/30 p-4 rounded-lg mt-4">
                                                <p class="text-sm italic">Note: A GitHub account is required to report issues.</p>
                                            </div>
                                        </div>
                                    </div>
                                {:else if item.id === 'item-2'}
                                    <div transition:slide={{duration: 200, easing: cubicOut}} class="px-6 py-4 bg-muted/20">
                                        <div class="space-y-4">
                                            <p class="text-muted-foreground">When all addons stop working, it's typically due to a corrupted addon_settings file.</p>
                                            
                                            <div class="bg-muted/30 p-4 rounded-lg space-y-3">
                                                <div class="flex items-start gap-2">
                                                    <div class="w-2 h-2 mt-2 rounded-full bg-green-400"></div>
                                                    <p class="flex-1"><span class="font-medium text-green-400">Recommended:</span> Try resetting your addon settings first</p>
                                                </div>
                                                <div class="flex items-start gap-2">
                                                    <div class="w-2 h-2 mt-2 rounded-full bg-red-400"></div>
                                                    <p class="flex-1"><span class="font-medium text-red-400">Last resort:</span> Uninstall all addons and reset settings if the first option doesn't work</p>
                                                </div>
                                            </div>

                                            <div class="flex gap-3 mt-4">
                                                <Button class="w-1/2" onclick={handleResetAddonSettings}>
                                                    Reset addon settings
                                                </Button>
                                                <Button variant="destructive" class="w-1/2" onclick={handleUninstallAllAddons}>
                                                    Uninstall all addons
                                                </Button>
                                            </div>
                                        </div>
                                    </div>
                                {:else}
                                    <div transition:slide={{duration: 200, easing: cubicOut}} class="px-6 py-4 bg-muted/20">
                                        {#if issueCount === 0}
                                            <div class="flex items-center justify-center py-8">
                                                <p class="text-green-400 font-medium">No issues detected in your system! 🎉</p>
                                            </div>
                                        {:else}
                                            <ScrollArea class="rounded-lg border border-zinc-700">
                                                {#each Object.keys(groupedIssues) as addonName}
                                                    <Collapsible.Root class="border-b border-zinc-700 last:border-0">
                                                        <Collapsible.Trigger class="w-full">
                                                            <div class="flex items-center justify-between p-4 hover:bg-zinc-900/50 transition-colors">
                                                                <div class="flex items-center gap-4 w-full">
                                                                    <h4 class="font-medium">{addonName === 'x2ui' ? 'Addon API' : addonName}</h4>
                                                                    <Badge variant="destructive" class="ml-auto mr-2">
                                                                        {groupedIssues[addonName].length} {groupedIssues[addonName].length === 1 ? 'issue' : 'issues'}
                                                                    </Badge>
                                                                </div>
                                                                <div class="transition-transform duration-200 data-[state=open]:rotate-180">
                                                                    <ChevronsUpDown size={20}/>
                                                                </div>
                                                            </div>
                                                        </Collapsible.Trigger>
                                                        <Collapsible.Content>
                                                            <div transition:slide={{duration: 200, easing: cubicOut}} class="space-y-2 p-4 bg-zinc-900/30">
                                                                {#each groupedIssues[addonName] as issue}
                                                                    <div class="rounded-md border border-zinc-700 px-4 py-3 font-mono text-sm bg-zinc-900/50">
                                                                        <p class="text-red-400 mb-1">Error: {issue.error}</p>
                                                                        <p class="text-zinc-400">File: <span class="text-indigo-400">{issue.file}</span></p>
                                                                    </div>
                                                                {/each}
                                                            </div>
                                                        </Collapsible.Content>
                                                    </Collapsible.Root>
                                                {/each}
                                            </ScrollArea>
                                        {/if}
                                    </div>
                                {/if}
                            </Accordion.Content>
                        </Accordion.Item>
                    {/each}
                </Accordion.Root>
            </div>
        </div>
    </main>
</div>