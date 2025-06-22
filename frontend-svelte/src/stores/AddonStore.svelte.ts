import type { Addon, Release } from "$lib/wails";
import { RemoteAddonService } from "$lib/wails";
import { safeCall } from "../utils";

let installedAddons: Array<Addon> = $state([]);
let updatesAvailableCount: number = $state(0);
let isCheckingForUpdates = $state(false);
let latestReleasesMap = $state(new Map<string, Release>());

export function setAddons(addons: Array<Addon>) {
    installedAddons = addons;
}

export function getInstalledAddons(): Array<Addon> {
    return installedAddons;
}

export function getUpdatesAvailableCount(): number {
    return updatesAvailableCount;
}

export function setUpdatesAvailableCount(count: number) {
    updatesAvailableCount = count;
}

export function getIsCheckingForUpdates(): boolean {
    return isCheckingForUpdates;
}

export function getLatestReleasesMap(): Map<string, Release> {
    return latestReleasesMap;
}

export async function performBulkUpdateCheck(): Promise<void> {
    if (isCheckingForUpdates) {
        return;
    }

    isCheckingForUpdates = true;
    try {
        const managedAddons = installedAddons.filter(addon => addon.isManaged);
        const addonNames = managedAddons.map(addon => addon.name);

        // Reset state if no managed addons
        if (addonNames.length === 0) {
            latestReleasesMap = new Map();
            updatesAvailableCount = 0;
            return;
        }

        // Get updates and handle potential errors
        const [releases, err] = await safeCall(RemoteAddonService.CheckAddonUpdatesBulk(addonNames));
        if (err || !releases) {
            console.error("[AddonStore] Failed to perform bulk update check:", err);
            latestReleasesMap = new Map();
            updatesAvailableCount = 0;
            return;
        }

        // Create map of latest releases and count updates
        latestReleasesMap = new Map(Object.entries(releases));
        updatesAvailableCount = managedAddons.reduce((count, addon) => {
            const latestRelease = latestReleasesMap.get(addon.name);
            return latestRelease?.published_at > addon.updatedAt ? count + 1 : count;
        }, 0);
    } catch (error) {
        console.error("[AddonStore] Unexpected error caught in performBulkUpdateCheck:", error);
        latestReleasesMap = new Map();
        updatesAvailableCount = 0;
    } finally {
        isCheckingForUpdates = false;
    }
}