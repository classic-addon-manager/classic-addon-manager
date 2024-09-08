<script lang="ts">
    import AppLoaderBusy from "./AppLoaderBusy.svelte";
    import {IsAddonInstalled} from "../../wailsjs/go/main/App";
    import {afterUpdate, onMount} from "svelte";
    import addons from "../addons";
    import type {addon as addonType} from "../../wailsjs/go/models";
    import ErrorBar from "./ErrorBar.svelte";

    export let addon: addonType.AddonManifest;

    let cmdbarExpanded = false;
    let isInstalled = false;
    let isDownloading = false;
    let errorMessage = '';

    onMount(async() => {
        isInstalled = await IsAddonInstalled(addon.name);
    })

    async function handleInstallClick() {
        if(isInstalled || isDownloading) return;
        isDownloading = true;

        try {
            isInstalled = await addons.install(addon)
        } catch(e) {
            if(e == 'no release found') {
                errorMessage = "No release found for this addon.";
            }
            isInstalled = false;
        }
        isDownloading = false;
    }

    // This is a hack to avoid the addon being marked as installed when it's not during a search
    afterUpdate(async () => {
        if(cmdbarExpanded) return;
        if(isInstalled) {
            isInstalled = await IsAddonInstalled(addon.name);
        }
    });
</script>

<div class="card">
    <div class="card-header">
        <div class="app-flex" style="justify-content: space-between; align-items: center">
            <h2>{addons.nameToDisplayName(addon.name)} <span style="font-weight: normal; font-size: 1rem">by {addon.author}</span></h2>

            <div class="app-cmd-bar">
                <button class="app-cmdbar-button" on:click={handleInstallClick}>
                    {#if !isInstalled && !isDownloading}
                        <i class="icons10-download app-color-primary"></i>
                        <span>Install</span>
                    {/if}
                    {#if !isInstalled && isDownloading}
                        <AppLoaderBusy />
                    {/if}

                    {#if isInstalled && !isDownloading}
                        <i class="icons10-checkmark app-color-success"></i>
                        <span>Installed</span>
                    {/if}
                </button>
                <div class="app-cmdbar-menu">
                    <div class="app-cmdbar-menu-trigger" data-win-toggle="dropdown" data-win-target="#MyMenu">
                        <button class="app-cmdbar-button" on:click={() => cmdbarExpanded = !cmdbarExpanded} on:cancel={() => cmdbarExpanded = false}>
                            <i class="icons10-angle-down"></i>
                        </button>
                    </div>
                    <ul class="app-cmdbar-menu-list left-justify" class:show={cmdbarExpanded} id="MyMenu">
                        <!--<li class="cmdbar-menu-list-item"><a href="#">
                            COMING SOON
                            <i class="icons10-file"></i>Other Versions</a>
                        </li>-->
                        <li class="cmdbar-menu-list-item">
                            <a on:click={() => cmdbarExpanded = false} href={`https://github.com/${addon.repo}/issues/new`} target="_blank"><i class="icons10-exclamation-mark"></i>Report Issue</a>
                        </li>
                        <li class="cmdbar-menu-list-item">
                            <a on:click={() => cmdbarExpanded = false} href={`https://github.com/${addon.repo}`} target="_blank"><i class="icons10-code-file"></i>View Code</a>
                        </li>
                    </ul>
                </div>
            </div>
        </div>
    </div>

    {#if errorMessage}
        <div style="margin-bottom: 1rem">
            <ErrorBar bind:message={errorMessage} />
        </div>
    {/if}

    <div class="app-flex" style="gap: 0.7rem; flex-wrap: wrap">
        {#each addon.tags as tag}
            <div class="badge">{tag}</div>
        {/each}
    </div>
    <p style="text-wrap: auto">{addon.description}</p>
</div>
