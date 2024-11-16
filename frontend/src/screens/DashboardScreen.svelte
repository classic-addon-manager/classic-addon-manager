<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { setActiveScreen } from "../stores/ScreenStore.svelte";
    import BrowseAddonsScreen from "./BrowseAddonsScreen.svelte";

    let searchName: string = $state("");

    import Search from "lucide-svelte/icons/search";
    import Refresh from "lucide-svelte/icons/refresh-cw";

    import { Input } from "$lib/components/ui/input/index.js";
    import { setActiveNavbar } from "$stores/NavbarStore.svelte";
    import * as Table from "$lib/components/ui/table/index.js";
    import LoaderCircle from "lucide-svelte/icons/loader-circle";

    import { getInstalledAddons } from "$stores/AddonStore.svelte";
    import { addon } from "../../wailsjs/go/models";
    import { setUpdatesAvailableCount } from "$stores/AddonStore.svelte";

    import LocalAddon from "../components/LocalAddon.svelte";
    import { onMount } from "svelte";

    let localAddons: Array<addon.Addon> = $state([]);
    let isCheckingForUpdates = $state(false);

    onMount(() => {
        setUpdatesAvailableCount(0);
    });

    $effect(() => {
        localAddons = getInstalledAddons();
    });

    let filteredAddons = $derived.by(() => {
        return localAddons.filter((addon) => {
            if (searchName === "") {
                return true;
            }
            return addon.name.toLowerCase().includes(searchName.toLowerCase());
        });
    });

    function checkForUpdates(): void {
        if (isCheckingForUpdates) {
            return;
        }
        setUpdatesAvailableCount(0);
        isCheckingForUpdates = true;
        const event = new CustomEvent("check-updates");
        document.dispatchEvent(event);
        // Allow some time to pass before button is enabled again.
        setTimeout(() => {
            isCheckingForUpdates = false;
        }, 1000);
    }
</script>

<header class="bg-muted/40 flex h-14 items-center gap-4 border-b px-4">
    <div class="w-full flex-1">
        <div class="relative">
            <Search
                class="text-muted-foreground absolute left-2.5 top-2.5 h-4 w-4"
            />
            <Input
                type="search"
                placeholder="Search installed addons..."
                class="bg-background w-full appearance-none pl-8 shadow-none"
                bind:value={searchName}
            />
        </div>
    </div>
</header>

<main class="flex flex-col gap-4 p-4">
    <div class="flex items-center">
        <h1 class="text-lg font-semibold md:text-2xl">Dashboard</h1>
        <div class="ml-auto">
            <Button onclick={checkForUpdates} disabled={isCheckingForUpdates}>
                {#if isCheckingForUpdates}
                    <LoaderCircle class="mr-2 size-4 animate-spin" />
                    Checking...
                {:else}
                    <Refresh /> Check For Updates
                {/if}
            </Button>
        </div>
    </div>
    {#if localAddons.length > 0}
        <div
            class="flex h-[calc(100vh-9rem)] rounded-lg border border-dashed shadow-sm px-3 py-2 overflow-auto"
            data-x-chunk-name="dashboard-02-chunk-1"
            data-x-chunk-description="This is the empty state for the dashboard screen"
        >
            <Table.Root>
                <Table.Header>
                    <Table.Row>
                        <Table.Head>Name</Table.Head>
                        <Table.Head class="text-center">Author</Table.Head>
                        <Table.Head class="text-center">Version</Table.Head>
                        <Table.Head class="text-center">Status</Table.Head>
                    </Table.Row>
                </Table.Header>
                <Table.Body>
                    {#each filteredAddons as addon (addon.name)}
                        <LocalAddon {addon} />
                    {/each}
                </Table.Body>
            </Table.Root>
        </div>
    {:else}
        <div
            class="flex flex-1 items-center justify-center rounded-lg border border-dashed shadow-sm"
            data-x-chunk-name="dashboard-02-chunk-1"
            data-x-chunk-description="This is the empty state for the dashboard screen"
        >
            <div class="flex flex-col items-center gap-1 text-center">
                <h3 class="text-2xl font-bold tracking-tight">
                    You have no addons
                </h3>
                <p class="text-muted-foreground text-sm">
                    You can browse addons from our repository by clicking the
                    button below.
                </p>
                <Button
                    class="mt-4"
                    onclick={() => {
                        setActiveScreen(BrowseAddonsScreen);
                        setActiveNavbar("addons");
                    }}>Browse addons</Button
                >
            </div>
        </div>
    {/if}
</main>
