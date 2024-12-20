import {
    GetAddOns as GoGetAddOns,
    UninstallAddon as GoUninstallAddon,
    InstallAddon as GoInstallAddon, GetAddonManifests as GoGetAddonManifests
} from "../wailsjs/go//app/App";
import {setAddons, getInstalledAddons} from "./stores/AddonStore.svelte";
import type {addon} from "../wailsjs/go/models";

export default {
    populateAddonStore,
    install,
    uninstall,
    repoHasAddon,
    getManifest,
    getInstalledAddons
}

async function getManifest(name: string): Promise<addon.AddonManifest> {
    const manifests = await GoGetAddonManifests();
    const m = manifests.find((m) => m.name === name);
    if (!m) {
        throw new Error("Addon not found");
    }
    return m;
}

async function repoHasAddon(addon: string): Promise<boolean> {
    const manifests = await GoGetAddonManifests();
    return manifests.some((m) => m.name === addon);
}

async function install(manifest: addon.AddonManifest): Promise<boolean> {
    const result = await GoInstallAddon(manifest);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}

async function populateAddonStore(): Promise<void> {
    const addons = await GoGetAddOns();
    setAddons(addons);
}

async function uninstall(addon: string): Promise<boolean> {
    const result = await GoUninstallAddon(addon);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}