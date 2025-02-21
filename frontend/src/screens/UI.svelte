<script lang="ts">
    import aacLogo from "../assets/images/aac-logo-wide.png";
    import semver from "semver";

    import {Button} from "$lib/components/ui/button/index.js";
    import * as Card from "$lib/components/ui/card/index";
    import {getActiveScreen} from "../stores/ScreenStore.svelte";
    import DashboardItem from "../components/navbar/DashboardItem.svelte";
    import AddonsItem from "../components/navbar/AddonsItem.svelte";
    import AACWebsiteItem from "../components/navbar/AACWebsiteItem.svelte";

    import {GetLatestApplicationRelease as GoGetLatestApplicationRelease} from "../../wailsjs/go/app/App";

    import {onMount} from "svelte";
    import TroubleshootingItem from "../components/navbar/TroubleshootingItem.svelte";
    import UserBar from "../components/UserBar.svelte";
    import {BrowserOpenURL} from "../../wailsjs/runtime";
    import {getVersion} from "$stores/ApplicationStore.svelte";

    let updateAvailable = $state(false);
    let updateInformation = $state({
        version: "",
        url: "",
    });

    let ActiveScreen = $derived.by(() => getActiveScreen())

    onMount(async () => {
        try {
            updateInformation = await GoGetLatestApplicationRelease();
        } catch (e) {
            console.error("Failed to get latest release: ", e);
            return;
        }

        if (isNewerVersion(updateInformation.version)) {
            updateAvailable = true;
        }
    });

    function isNewerVersion(version: string): boolean {
        return semver.gt(version, getVersion());
    }
</script>

<div class="grid min-h-screen w-full md:grid-cols-[220px_1fr]">
    <div class="bg-muted/40 border-r">
        <div class="flex h-full max-h-screen flex-col gap-2">
            <div class="flex h-14 items-center border-b px-4">
                <div class="flex items-center gap-2 font-semibold">
                    <img
                            src={aacLogo}
                            alt="ArcheAge Classic Logo"
                            class="h-12 w-auto"
                    />
                </div>
            </div>
            <div class="flex-1">
                <nav class="grid items-start px-2 text-sm font-medium">
                    <DashboardItem/>
                    <AddonsItem/>
                    <TroubleshootingItem/>
                    <AACWebsiteItem/>
                </nav>
            </div>

            {#if updateAvailable}
                <div class="mt-auto p-4">
                    <Card.Root
                            data-x-chunk-name="dashboard-02-chunk-0"
                            data-x-chunk-description="A card with a call to action"
                    >
                        <Card.Header class="p-2 pt-0 md:p-4">
                            <Card.Title>Update available!</Card.Title>
                            <Card.Description>
                                Version {updateInformation.version} is available.
                                <br/>
                                <br/>
                                Click the button below to download it.
                            </Card.Description>
                        </Card.Header>
                        <Card.Content class="p-2 pt-0 md:p-4 md:pt-0">
                            <Button
                                    size="sm"
                                    class="w-full"
                                    onclick={() => BrowserOpenURL(updateInformation.url)}
                            >Download
                            </Button>
                        </Card.Content>
                    </Card.Root>
                </div>
            {/if}

            <div class="w-full">
                <UserBar/>
            </div>

            <div class="mx-auto mb-2 text-gray-300 text-opacity-40">
                <span class="hover:text-blue-400 cursor-pointer transition-all"
                      onclick={() => BrowserOpenURL(`https://github.com/classic-addon-manager/classic-addon-manager/releases/tag/v${getVersion()}`)}>v{getVersion()}
                    by Sami</span>
            </div>
        </div>
    </div>
    <div class="flex flex-col">
        <ActiveScreen/>
    </div>
</div>
