<!-- @migration-task Error while migrating Svelte code: Can't migrate code with afterUpdate. Please migrate by hand. -->
<script lang="ts">
    import AppLoaderBusy from "./AppLoaderBusy.svelte";
    import {IsAddonInstalled} from "../../wailsjs/go/main/App";
    import {onMount} from "svelte";
    import addons from "../addons";
    import type {addon as addonType} from "../../wailsjs/go/models";
    import ErrorBar from "./ErrorBar.svelte";
    import doodleru from '../assets/images/doodleru.png';
    
    let {addon}: {addon: addonType.AddonManifest} = $props();
    
    let cmdbarExpanded = $state(false);
    let isInstalled = $state(false);
    let isDownloading = $state(false);
    let errorMessage = $state('');
    
    let hasBanner = $state(false);
    let banner: string = $state('');
    
    onMount(async() => {
        isInstalled = await IsAddonInstalled(addon.name);
        hasBanner = await checkForBanner();
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
    
    async function checkForBanner() {
        const response = await fetch(`https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/banner.png`);
        if(!response.ok) {
            return false;
        }
        
        const reader = response.body?.getReader();
        if(reader) {
            const chunks = [];
            let done = false;
            while(!done) {
                let result;
                try {
                    result = await reader.read();
                } catch (e) {
                    console.error("Error reading image:", e);
                    return false;
                }
                done = result.done;
                if(result.value) {
                    chunks.push(result.value);
                }
            }
            const blob = new Blob(chunks);
            banner = URL.createObjectURL(blob);
        }
        return true;
    }
</script>

<div class="card">
    <div class="card-header">
        <div class="app-flex" style="justify-content: space-between; align-items: center">
            <h2 style="flex-grow: 1">{addons.nameToDisplayName(addon.name)} <span style="font-weight: normal; font-size: 1rem; white-space: nowrap">by {addon.author}</span></h2>
            
            <div class="app-cmd-bar" style="align-self: flex-start; margin-top: 1rem">
                <button class="app-cmdbar-button" onclick={handleInstallClick}>
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
                        <button class="app-cmdbar-button" onclick={() => cmdbarExpanded = !cmdbarExpanded} oncancel={() => cmdbarExpanded = false} aria-label="Toggle command bar">
                            <i class="icons10-angle-down"></i>
                        </button>
                    </div>
                    <ul class="app-cmdbar-menu-list left-justify" class:show={cmdbarExpanded} id="MyMenu">
                        <!--<li class="cmdbar-menu-list-item"><a href="#">
                        COMING SOON
                        <i class="icons10-file"></i>Other Versions</a>
                        </li>-->
                        <li class="cmdbar-menu-list-item">
                            <a onclick={() => cmdbarExpanded = false} href={`https://github.com/${addon.repo}/issues/new`} target="_blank"><i class="icons10-exclamation-mark"></i>Report Issue</a>
                        </li>
                        <li class="cmdbar-menu-list-item">
                            <a onclick={() => cmdbarExpanded = false} href={`https://github.com/${addon.repo}`} target="_blank"><i class="icons10-code-file"></i>View Code</a>
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
    
    <div style="width: 100%; border-radius: 0.5rem; margin-top: 1rem; max-height: 360px; overflow: hidden">
        {#if hasBanner}
            <img src={banner} alt="Banner" style="width: 100%; height: auto; object-fit: cover;"/>
        {:else}
            <img src={doodleru} alt="Banner" style="width: 100%; height: auto; object-fit: cover; opacity: 25%; filter: blur(1px)"/>
        {/if}
    </div>
    
    <p style="text-wrap: auto">{addon.description}</p>
</div>
