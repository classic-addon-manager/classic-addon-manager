import type {Addon} from '$lib/wails'

let drawerOpen = $state(false)
let selectedAddon = $state<Addon | null>(null)

export function openAddonDrawer(addon: Addon) {
  selectedAddon = addon
  drawerOpen = true
}


export function getDrawerState() {
  return {
    open: drawerOpen,
    addon: selectedAddon
  }
}

export function setDrawerOpen(open: boolean) {
  drawerOpen = open
  if (!open) {
    // Delay clearing the addon to allow the closing animation to complete
    setTimeout(() => {
      selectedAddon = null
    }, 300)
  }
}