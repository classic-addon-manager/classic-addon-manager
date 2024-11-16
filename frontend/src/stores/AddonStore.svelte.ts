import type { addon } from "../../wailsjs/go/models";

let installedAddons: Array<addon.Addon> = $state([]);
// export const installedAddons = writable([]);

export function setAddons(addons: Array<addon.Addon>) {
    installedAddons = addons;
}

export function getInstalledAddons(): Array<addon.Addon> {
    return installedAddons;
}