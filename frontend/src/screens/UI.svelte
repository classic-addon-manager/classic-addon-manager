<script lang="ts">
    import aacLogo from "../assets/images/aac-logo-wide.png";
    import packageJson from "../../package.json";

    import { Button } from "$lib/components/ui/button/index.js";
    import * as Card from "$lib/components/ui/card/index.js";
    import { getActiveScreen } from "../stores/ScreenStore.svelte";
    import DashboardItem from "../components/navbar/DashboardItem.svelte";
    import AddonsItem from "../components/navbar/AddonsItem.svelte";
    import AACWebsiteItem from "../components/navbar/AACWebsiteItem.svelte";
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
                    <DashboardItem />
                    <AddonsItem />
                    <AACWebsiteItem />
                </nav>
            </div>
            <div class="mt-auto p-4">
                <Card.Root
                    data-x-chunk-name="dashboard-02-chunk-0"
                    data-x-chunk-description="A card with a call to action"
                >
                    <Card.Header class="p-2 pt-0 md:p-4">
                        <Card.Title>Update available!</Card.Title>
                        <Card.Description
                            >TODO: Show some information about the new release</Card.Description
                        >
                    </Card.Header>
                    <Card.Content class="p-2 pt-0 md:p-4 md:pt-0">
                        <Button size="sm" class="w-full">Download</Button>
                    </Card.Content>
                </Card.Root>
            </div>

            <div class="mx-auto mb-2 text-gray-300 text-opacity-40">
                <span>v{packageJson.version} by Sami</span>
            </div>
        </div>
    </div>
    <div class="flex flex-col">
        <svelte:component this={getActiveScreen()} />
    </div>
</div>
