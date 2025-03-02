import type { Addon} from "$lib/wails";

let installedAddons: Array<Addon> = $state([]);
let updatesAvailableCount: number = $state(0);
let localAddonDialogOpen: boolean = $state(false);

export function setAddons(addons: Array<Addon>) {
    installedAddons = addons;
}

export function getInstalledAddons(): Array<Addon> {
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

export function getLocalAddonDialogOpen(): boolean {
    return localAddonDialogOpen;
}