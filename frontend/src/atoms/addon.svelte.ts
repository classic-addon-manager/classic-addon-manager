import {Addon, Release} from '$lib/wails'

let installed: Array<Addon> = $state([])
let updateCount: number = $state(0)
let updating: boolean = $state(false)
let latestReleaseMap = $state(new Map<string, Release>())

export function setInstalledAddons(addons: Array<Addon>): void {
  installed = addons
}

export function getInstalledAddons(): Array<Addon> {
  return installed
}

export function getUpdateCount(): number {
  return updateCount
}

export function setUpdateCount(count: number): void {
  updateCount = count
}

export function isCheckingUpdates(): boolean {
  return updating
}

export function setCheckingUpdates(checking: boolean): void {
  updating = checking
}

export function getLatestReleaseMap(): Map<string, Release> {
  return latestReleaseMap
}

export function setLatestReleaseMap(map: Map<string, Release>): void {
  latestReleaseMap = map
}
