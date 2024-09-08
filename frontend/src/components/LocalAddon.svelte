<script lang="ts">
    import {addon as ad, api} from "../../wailsjs/go/models";
    import {crossfade} from "svelte/transition";
    import Dialog from "./Dialog.svelte";
    import addons from "../addons";
    import ErrorBar from "./ErrorBar.svelte";
    import {onDestroy, onMount} from "svelte";
    import {GetLatestRelease as GoGetLatestRelease} from "../../wailsjs/go/main/App";
    import AppLoaderBusy from "./AppLoaderBusy.svelte";

    export let addon: ad.Addon;

    let dialog: any;
    let dialogErrorMsg = '';

    let uninstallClicks = 0;
    let uninstallTimeout: any;

    let latestRelease: api.Release;
    let isCheckingForUpdates = false;

    onMount(async() => {
        if(addon.isManaged) {
            document.addEventListener('check-updates', handleCheckUpdates);
            await handleCheckUpdates();
        }
    });

    onDestroy(() => {
        document.removeEventListener('check-updates', handleCheckUpdates);
    });

    async function handleCheckUpdates() {
        isCheckingForUpdates = true;
        console.log("Checking for updates for addon: ", addon.name);
        let release: api.Release = null;
        try {
            release = await GoGetLatestRelease(addon.name)
        } catch(e) {
            console.error("Failed to get release for addon: ", addon.name);
            isCheckingForUpdates = false;
            return;
        }
        if(!release) {
            console.error("No release found for addon: ", addon.name);
            return;
        }
        latestRelease = release;
        isCheckingForUpdates = false;
    }

    async function handleUninstall() {
        uninstallClicks++;

        uninstallTimeout = setTimeout(() => {
            uninstallClicks = 0;
        }, 5000);

        if(uninstallClicks === 2) {
            clearTimeout(uninstallTimeout);
            uninstallClicks = 0;
            console.debug("Uninstalling addon: ", addon.name);
            dialog.toggle();
            let didUninstall = await addons.uninstall(addon.name);
            if(didUninstall) {
                console.debug("Uninstalled addon: ", addon.name);
            } else {
                console.error("Failed to uninstall addon: ", addon.name);
            }
        }
    }

    async function handleMatchAddon() {
        let manifest = await addons.getManifest(addon.name)
        if(!manifest) {
            console.error('Failed to get manifest, this should not happen...')
            return;
        }

        try {
            await addons.install(manifest)
        } catch(e) {
            if(e.toString().includes('no release found')) {
                dialogErrorMsg = "No release found for this addon.";
            }
            return;
        }

        dialog.toggle();
    }

    async function handleForceUpdate() {
        let didInstall = false;
        try {
            didInstall = await addons.install(await addons.getManifest(addon.name))
        } catch(e) {
            if(e.toString().includes('no release found')) {
                dialogErrorMsg = "No release found for this addon.";
            } else {
                dialogErrorMsg = "Failed to update addon.";
            }
            return;
        }
        if(!didInstall) return;
        dialog.toggle();
    }

    // For now this is just a duplicate of handleForceUpdate
    async function handleUpdateClick() {
        let didInstall = false;
        try {
            didInstall = await addons.install(await addons.getManifest(addon.name))
        } catch(e) {
            if(e.toString().includes('no release found')) {
                // dialogErrorMsg = "No release found for this addon.";
            } else {
                // dialogErrorMsg = "Failed to update addon.";
            }
            return;
        }
        if(!didInstall) return;
    }
</script>

<tr style="user-select: none; height: 46px" on:click={dialog.toggle()}>
    <td style="text-align: left">{addon.displayName}</td>
    <td>{addon.author || ''}</td>
    <td>{addon.version}</td>
    <td>
        {#if addon.isManaged}
            {#if isCheckingForUpdates}
                <AppLoaderBusy />
            {:else if latestRelease}
                {#if latestRelease.published_at > addon.updatedAt}
                    <button class="app-btn app-btn-primary" on:click|preventDefault|stopPropagation={handleUpdateClick}>
                        <span class="icons10-refresh"></span>
                        Update ({latestRelease.tag_name})
                    </button>
                {:else}
                    <span class="app-color-success icons10-checkmark"></span>
                    Up to date
                {/if}
            {:else}
                <span class="app-color-success icons10-checkmark"></span>
                Up to date
            {/if}


        {:else}
            <button class="app-btn app-btn-warning">
                <span class="icons10-cross"></span>
                Not managed
            </button>
        {/if}
    </td>
</tr>

<Dialog header="{addon.displayName}" bind:this={dialog}>
    <div slot="body" class="px-md" style="text-align: left">
        {#if addon.isManaged}
            <p>{addon.description}</p>
        {:else}
            <p>This addon is not managed by Classic Addon Manager.</p>

            {#await addons.repoHasAddon(addon.name)}
                <span></span>
            {:then found}
                {#if found}
                    <h3>Good news!</h3>
                    <p>The addon was found in our addon repository. Do you want Classic Addon Manager to manage updates?</p>
                    {#if dialogErrorMsg}
                        <div style="margin-bottom: 1rem;">
                            <ErrorBar bind:message={dialogErrorMsg} />
                        </div>
                    {/if}
                    <button style="margin-bottom: 1rem" class="app-btn app-btn-primary"
                        on:click={handleMatchAddon}
                    >Yes, do it!</button>
                {/if}
            {/await}
        {/if}
    </div>
    <div slot="footer">
        {#if dialogErrorMsg}
            <div style="margin-bottom: 1rem;">
                <ErrorBar bind:message={dialogErrorMsg} />
            </div>
        {/if}
        <div class="app-flex" style="justify-content: space-between">
            <div>
                {#if addon.isManaged}
                    <button class="app-btn app-btn-outline-primary" type="button" on:click={handleForceUpdate}>
                        <i class="icons10-refresh"></i><span>Force Reinstall</span>
                    </button>
                {/if}
                <button class="app-btn app-btn-outline-danger" type="button" on:click={handleUninstall}>
                    {#if uninstallClicks === 0}
                        <i class="icons10-trash"></i>
                        <span>
                            Uninstall
                        </span>
                    {:else}
                        <span>Click again</span>
                    {/if}
                </button>
            </div>

            <button style="" class="app-btn" type="button" on:click={dialog.toggle()}>
                <span>Close</span>
            </button>
        </div>

    </div>
</Dialog>

<style>
    tr:hover {
        cursor: pointer;
        background-color: #555555;
        transition: background-color 0.3s ease;
    }
</style>