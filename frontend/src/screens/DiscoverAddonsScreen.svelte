<script lang="ts">
    import {onMount} from "svelte";
    import AppLoaderBar from "../components/AppLoaderBar.svelte";
    import {GetAddonManifest} from "../../wailsjs/go/main/App";
    import RemoteAddon from "../components/RemoteAddon.svelte";
    import type {addon} from "../../wailsjs/go/models";
    import Dropdown from "../components/Dropdown.svelte";

    let isReady: boolean = $state(false);
    let addons: addon.AddonManifest[] = $state([]);
    let searchPhrase = $state('');
    let selectedTag = $state('all');
    let tags = $state([{label: 'All', value: 'all'}]);

    // Naive search, should be improved. Fuzzy? Relevancy?
    const filteredAddons = $derived.by(() => {
        return addons.filter(addon => {
            if(selectedTag != 'all' && !addon.tags.includes(selectedTag)) {
                return false;
            }
            // @ts-ignore
            return addon.displayName.toLowerCase().includes(searchPhrase.toLowerCase()) || addon.description.toLowerCase().includes(searchPhrase.toLowerCase());
        });
    });

    onMount(async () => {
        await _getAddonManifest();
        isReady = true;
    });

    async function _getAddonManifest() {
        let tmp = [];
        for(let addon of await GetAddonManifest(false)) {
            tmp.push({
                ...addon,
                displayName: addon.name.replace(/_/g, " ")
            });
            for(let tag of addon.tags) {
                // Skip the Example tag
                if(tag == 'Example') {
                    continue;
                }
                if(tags.find(t => t.value == tag)) {
                    continue;
                }
                tags.push({label: tag, value: tag});
            }
        }
        // Sort addons alphabetically by name
        tmp.sort((a, b) => a.name.localeCompare(b.name));
        tags.sort((a, b) => a.label.localeCompare(b.label));
        addons = tmp;
        searchPhrase = '';
    }

    async function refreshAddons() {
        isReady = false;
        await GetAddonManifest(true);
        await _getAddonManifest();
        isReady = true;
    }
</script>

<main style="width: 100%">
    <div style="width: 100%; justify-content: space-between; align-items: center" class="app-flex">
        <h1>Get More Addons</h1>
        <button class="app-btn app-btn-outline-primary"
                class:input-field-default-disabled={!isReady}
                disabled={!isReady}
            onclick={refreshAddons}
        ><i class="icons10-refresh font-size-16px"></i>Refresh Addons</button>
    </div>

    {#if !isReady}
        <div class="app-flex fill-parent" style="flex-flow: column; text-align: center">
            <p>Loading addons...</p>
            <AppLoaderBar />
        </div>
    {:else}
        <div class="app-flex-col" style="margin-bottom: 0rem; width: 100%">
            
            <div class="app-input-search-bar fill-parent">
                <div style="margin-right: 0.5rem">
                    <Dropdown valueChanged={(tag) => {
                    selectedTag = tag;
                }} options={tags} />
                </div>
                
                <input class="app-input-text" type="search" placeholder="Search by name or description" bind:value={searchPhrase} />
                <div class="app-input-end-content">
                    <button type="submit" aria-label="Search"></button>
                </div>
            </div>

            <div class="fill-parent">
                <p class="app-color-grey">Showing {filteredAddons.length} of {addons.length} addons</p>
            </div>
        </div>

        <div class="addon-grid-container" style="margin-bottom: 1rem">
            {#each filteredAddons as addon(addon.name)}
                <RemoteAddon addon={addon} />
            {:else}
                <div class="app-flex-center"><h3>No addons found</h3></div>
            {/each}
        </div>
    {/if}
</main>