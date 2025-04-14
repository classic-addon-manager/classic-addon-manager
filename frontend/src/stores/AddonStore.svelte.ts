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
    let newUpdatesCount = 0;

    try {
        const managedAddons = installedAddons.filter(addon => addon.isManaged);
        const addonNames = managedAddons.map(addon => addon.name);

        if (addonNames.length === 0) {
            latestReleasesMap = new Map();
            updatesAvailableCount = 0;
        } else {
            const [releases, err] = await safeCall(RemoteAddonService.CheckAddonUpdatesBulk(addonNames));

            if (err || !releases) {
                console.error("[AddonStore] Failed to perform bulk update check:", err);
                latestReleasesMap = new Map();
            } else {
                const releasesFrontendMap = new Map<string, Release>();
                for (const name in releases) {
                    if (Object.prototype.hasOwnProperty.call(releases, name)) {
                        releasesFrontendMap.set(name, releases[name]);
                    }
                }
                latestReleasesMap = releasesFrontendMap;

                managedAddons.forEach(addon => {
                    const latestRelease = latestReleasesMap.get(addon.name);
                    if (latestRelease && latestRelease.published_at > addon.updatedAt) {
                        newUpdatesCount++;
                    }
                });
            }
        }
    } catch (error) {
        console.error("[AddonStore] Unexpected error caught in performBulkUpdateCheck:", error);
        latestReleasesMap = new Map();
    } finally {
        updatesAvailableCount = newUpdatesCount;
        isCheckingForUpdates = false;
    }
}