<script lang="ts">
    import { installedAddons } from "../stores/AddonStore.svelte";
    import LocalAddon from "../components/LocalAddon.svelte";
    import { onMount } from "svelte";
    import AppLoaderBusy from "../components/AppLoaderBusy.svelte";

    let localAddonRefs = $state([]);
    onMount(() => {
        localAddonRefs = [];
    });

    function checkAllUpdates() {
        if (isCheckingUpdates) {
            return;
        }
        isCheckingUpdates = true;
        const event = new CustomEvent("check-updates");
        document.dispatchEvent(event);
        isCheckingUpdates = false;
    }

    let isCheckingUpdates = $state(false);
</script>

<main style="width: 100%;">
    <div
        class="app-flex"
        style="justify-content: space-between; align-items: center"
    >
        <h1>Addons ({$installedAddons.length})</h1>
        <div>
            <button
                style="width: 173px"
                class="app-btn app-btn-outline-primary"
                onclick={checkAllUpdates}
            >
                {#if isCheckingUpdates}
                    <div
                        class="fill-parent app-flex"
                        style="align-items: center; justify-content: center"
                    >
                        <AppLoaderBusy />
                    </div>
                {:else}
                    <span>
                        <span class="icons10-refresh"></span>
                        Check For Updates
                    </span>
                {/if}
            </button>
        </div>
    </div>
    <div
        class="app-table-view-container"
        style="max-width: none; margin-bottom: 1rem;"
    >
        <table class="app-table-view" style="width: 100%">
            <thead>
                <tr style="text-align: center">
                    <th style="text-align: left">Name</th>
                    <th style="text-align: center">Author</th>
                    <th>Version</th>
                    <th>Status</th>
                </tr>
            </thead>
            <tbody style="text-align: center">
                {#each $installedAddons as addon, index}
                    <LocalAddon bind:this={localAddonRefs[index]} {addon} />
                {/each}
            </tbody>
        </table>
    </div>
</main>
