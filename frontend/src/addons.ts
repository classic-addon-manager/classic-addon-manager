import {
    LocalAddonService,
} from "$lib/wails"
import {setAddons, getInstalledAddons} from "./stores/AddonStore.svelte";
import type {AddonManifest} from "$lib/wails";
import {RemoteAddonService} from "$lib/wails";
import { safeCall, toast } from "./utils";

export default {
    populateAddonStore,
    install,
    uninstall,
    repoHasAddon,
    getManifest,
    getInstalledAddons,
    unmanage,
    getSubscribedAddons
}

async function getManifest(name: string): Promise<AddonManifest> {
    const manifests = await RemoteAddonService.GetAddonManifest();
    const m = manifests.find((m) => m.name === name);
    if (!m) {
        throw new Error(`Addon not found: ${name}`);
    }
    return m;
}

async function repoHasAddon(addon: string): Promise<boolean> {
    const manifests = await RemoteAddonService.GetAddonManifest();
    return manifests.some((m) => m.name === addon);
}

async function install(manifest: AddonManifest): Promise<boolean> {
    const result = await RemoteAddonService.InstallAddon(manifest);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}

async function populateAddonStore(): Promise<void> {
    setAddons((await LocalAddonService.GetAddOns()));
}

async function uninstall(addon: string): Promise<boolean> {
    const result = await LocalAddonService.UninstallAddon(addon);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}

async function unmanage(addon: string): Promise<boolean> {
    const result = await LocalAddonService.UnmanageAddon(addon);
    if (result) {
        await populateAddonStore();
        return true;
    }
    return false;
}

async function getSubscribedAddons() {
    const [ads, err] = await safeCall<AddonManifest[]>(RemoteAddonService.GetSubscribedAddons());
    if(err) {
        toast.error("Error getting subscribed addons");
        console.error("Error getting subscribed addons", err);
        return;
    }

    if(!ads) {
        return;
    }

    let installed = 0;

    for(const addon of ads) {
        const [is_installed, err] = await safeCall<boolean>(LocalAddonService.IsInstalled(addon.name));
        if(err) {
            console.error("Error checking if addon is installed", err);
            toast.error(`Error occurred while checking if addon is installed: ${err}`);
            continue;
        }

        if(is_installed) {
            continue;
        }

        if(await install(addon)) {
            installed++;
        }
    }

    if(installed > 0) {
        toast.success("Addons restored", {description: `${installed} addons were restored from the server!`});
    }
}