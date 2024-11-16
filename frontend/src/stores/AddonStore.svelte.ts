import type { addon } from "../../wailsjs/go/models";

let installedAddons: Array<addon.Addon> = $state([]);
let updatesAvailableCount: number = $state(0);

export function setAddons(addons: Array<addon.Addon>) {
    installedAddons = addons;
}

export function getInstalledAddons(): Array<addon.Addon> {
    return installedAddons;
}

export function addUpdateAvailableCount() {
    updatesAvailableCount++;
}

export function getUpdatesAvailableCount(): number {
    return updatesAvailableCount;
}

export function setUpdatesAvailableCount(count: number) {
    updatesAvailableCount = count;
}