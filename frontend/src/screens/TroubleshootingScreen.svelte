<script lang="ts">
    import * as Card from "$lib/components/ui/card/index";
    import * as Accordion from "$lib/components/ui/accordion/index";
    import {Badge} from "$lib/components/ui/badge/index";
    import {Button} from "$lib/components/ui/button/index";
    import {toast} from "svelte-sonner";
    import {
        ResetAddonSettings as GoResetAddonSettings,
        UninstallAddon as GoUninstallAddon,
        DiagnoseIssues as GoDiagnoseIssues
    } from "../../wailsjs/go/app/App.js";
    import {getInstalledAddons} from "$stores/AddonStore.svelte";
    import {addon, util} from "../../wailsjs/go/models.js";
    import addons from "../addons";
    import {onMount} from "svelte";

    let issues: util.LogParseResult[] = $state([]);
    let issueCount = $derived.by(() => issues.length);

    onMount(async () => {
        let res = await GoDiagnoseIssues();
        issues = res;
    });

    async function handleResetAddonSettings() {
        try {
            await GoResetAddonSettings();
        } catch (e) {
            console.error("Failed to reset addon settings", e);
            toast.error(`Failed to reset addon settings, reason: ${e}`, {duration: 6000});
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
                        TODO: Implement issue display
                    {/if}
                </Accordion.Content>
            </Accordion.Item>
        </Accordion.Root>
    </div>

</main>