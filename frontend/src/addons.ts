import {
    GetAddOns as GoGetAddOns,
    UninstallAddon as GoUninstallAddon,
    InstallAddon as GoInstallAddon, GetAddonManifests as GoGetAddonManifests
} from "../wailsjs/go/main/App";
import {installedAddons} from "./stores/AddonStore";
import type {addon} from "../wailsjs/go/models";

export default {
    populateAddonStore,
    install,
    uninstall,
    nameToDisplayName,
    repoHasAddon,
    getManifest
}

async function getManifest(name: string): Promise<addon.AddonManifest | null> {
    const manifests = await GoGetAddonManifests();
    return manifests.find((m) => m.name === name) || null;
}

function nameToDisplayName(name: string): string {
    return name.replaceAll("_", " ");
}

async function repoHasAddon(addon: string): Promise<boolean> {
    const manifests = await GoGetAddonManifests();
    return manifests.some((m) => m.name === addon);
}

async function install(manifest: addon.AddonManifest): Promise<boolean> {
    const result = await GoInstallAddon(manifest);
    if(result) {
        await populateAddonStore();
        return true;
    }
    return false;
}

async function populateAddonStore(): Promise<void> {
    const addons = await GoGetAddOns();
    installedAddons.set(addons);
}

async function uninstall(addon: string): Promise<boolean> {
    const result = await GoUninstallAddon(addon);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}