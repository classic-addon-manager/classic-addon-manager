<script lang="ts">
    import * as Card from "$lib/components/ui/card/index";
    import * as Accordion from "$lib/components/ui/accordion/index";
    import * as Collapsible from "$lib/components/ui/collapsible/index";
    import {ScrollArea} from "$lib/components/ui/scroll-area/index";
    import {Badge} from "$lib/components/ui/badge/index";
    import {Button} from "$lib/components/ui/button/index";
    import {
        ResetAddonSettings as GoResetAddonSettings,
        UninstallAddon as GoUninstallAddon,
        DiagnoseIssues as GoDiagnoseIssues,
        addon,
        util
    } from "$lib/wails";
    import {getInstalledAddons} from "$stores/AddonStore.svelte";
    import addons from "../addons";
    import {onMount} from "svelte";
    import {ChevronsUpDown} from "lucide-svelte";
    import {toast} from "../utils";

    let issues: util.LogParseResult[] = $state([]);
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
        issues = await GoDiagnoseIssues();
    });

    async function handleResetAddonSettings() {
        try {
            await GoResetAddonSettings();
        } catch (e) {
            console.error("Failed to reset addon settings", e);
            toast.error(`Failed to reset addon settings, reason: ${e}`, {
                duration: 6000,
            });
            return;
        }
        toast.error("Addon settings have been reset, please restart the game");
    }

    async function handleUninstallAllAddons() {
        let ads: addon.Addon[] = getInstalledAddons();
        if (ads.length === 0) {
            toast.error("No addons to uninstall");
            return;
        }
        let uninstallCount = 0;
        for (let a of ads) {
            let uninstalled = await GoUninstallAddon(a.name);
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

<header class="bg-muted/40 flex h-14 items-center gap-4 border-b px-4">
    <div class="flex gap-2 w-full">
        <p class="text-muted-foreground">Select the issue you are having</p>
    </div>
</header>

<main class=" h-[calc(100vh-4rem)] overflow-auto">
    <div class="mt-2 mx-auto w-11/12">
        <Accordion.Root type="single">
            <Accordion.Item value="item-1">
                <Accordion.Trigger>A specific addon doesn't work</Accordion.Trigger>
                <Accordion.Content>
                    <p>In the event that a specific addon does not work as it should, you should go to Dashboard
                        right click the addon and select "Report issue".</p><br/>
                    <p>This will open up a new issue on the addon's github repository where you can describe the
                        problem. keep in mind that a github account is required to do so.</p>
                </Accordion.Content
                >
            </Accordion.Item>
            <Accordion.Item value="item-2">
                <Accordion.Trigger>No addons are working</Accordion.Trigger>
                <Accordion.Content>
                    <p>In cases where all addons stop working, it is likely the result of a borked addon_settings
                        file.</p><br/>
                    <p>
                        Below you can find two options, <span class="font-semibold text-green-400">consider first trying the option to reset your addon settings
                        and try again</span>.
                        If that doesn't solve your issue you may <span class="font-semibold text-red-400">click the other button which will uninstall all addons</span>
                        and reset your addon_settings file.
                        Once that is done you can reinstall your addons.
                    </p>

                    <Button class="mt-4" onclick={handleResetAddonSettings}>Reset addon settings</Button>
                    <Button variant="destructive" class="mt-4 ml-2" onclick={handleUninstallAllAddons}>Uninstall all
                        addons
                    </Button>
                </Accordion.Content>
            </Accordion.Item>
            <Accordion.Item value="item-3">
                <Accordion.Trigger>Diagnostic
                    {#if issueCount > 0}
                        <div class="ml-auto mr-5 no-underline">
                            <Badge variant="destructive">{issueCount} issues</Badge>
                        </div>
                    {:else}
                        <div class="ml-auto mr-5 no-underline">
                            <Badge variant="successful">
                                No issues found
                            </Badge>
                        </div>
                    {/if}
                </Accordion.Trigger>
                <Accordion.Content>
                    {#if issueCount === 0}
                        <p>No issues found, good job!</p>
                    {:else}
                        <ScrollArea class="rounded-md border p-4 border-zinc-700">
                            {#each Object.keys(groupedIssues) as addonName}
                                <Collapsible.Root class="space-y-2 py-2 hover:bg-zinc-900">
                                    <Collapsible.Trigger class="w-full">
                                        <div class="flex items-center justify-between space-x-4 px-4 cursor-pointer">
                                            <div class="flex w-full">
                                                <h4 class="text-sm font-semibold">{addonName === 'x2ui' ? 'Addon API' : addonName}</h4>
                                                <div class="ml-auto no-underline">
                                                    <Badge variant="destructive">{groupedIssues[addonName].length}
                                                        issues
                                                    </Badge>
                                                </div>
                                            </div>
                                            <Button variant="ghost" size="sm" class="w-9 p-0">
                                                <ChevronsUpDown/>
                                                <span class="sr-only">Toggle</span>
                                            </Button>
                                        </div>
                                    </Collapsible.Trigger>
                                    <Collapsible.Content class="space-y-2 px-3 pb-1">
                                        <!-- Issue content -->
                                        {#each groupedIssues[addonName] as issue}
                                            <div class="rounded-md border px-4 py-3 font-mono text-sm select-text">
                                                {issue.error} in
                                                <span class="font-semibold text-indigo-400 select-text">{issue.file}</span>
                                            </div>
                                        {/each}

                                    </Collapsible.Content>
                                </Collapsible.Root>
                                {#each groupedIssues[addonName] as issue}

                                {/each}
                            {/each}
                        </ScrollArea>

                    {/if}
                </Accordion.Content>
            </Accordion.Item>
        </Accordion.Root>
    </div>

</main>