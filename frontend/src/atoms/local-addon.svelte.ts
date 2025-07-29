import {type Addon, Release} from '$lib/wails'

interface DialogState {
    isOpen: boolean
    addon: Addon | null
}

interface UpdateDialog {
    isOpen: boolean
    addon: Addon | null
    release: Release | null
}

let dialogState = $state<DialogState>({
  isOpen: false,
  addon: null
})

let updateDialogState = $state<UpdateDialog>({
  isOpen: false,
  addon: null,
  release: null
})

export const addonDialog = {
  get isOpen() {
    return dialogState.isOpen
  },

  get addon() {
    return dialogState.addon
  },

  setOpen(open: boolean) {
    if (!open) {
      // Allow for animation to finish
      setTimeout(() => {
        dialogState.isOpen = false
        dialogState.addon = null
      }, 200)
    }
  },

  openWith(addon: Addon) {
    dialogState.addon = addon
    dialogState.isOpen = true
  }
}

export const updateDialog = {
  get isOpen() {
    return updateDialogState.isOpen
  },

  get addon() {
    return updateDialogState.addon
  },

  get release() {
    return updateDialogState.release
  },

  setOpen(open: boolean) {
    if (!open) {
      // Allow for animation to finish
      setTimeout(() => {
        updateDialogState.isOpen = false
        updateDialogState.addon = null
        updateDialogState.release = null
      }, 200)
    }
  },

  openWith(addon: Addon, release: Release) {
    updateDialogState.addon = addon
    updateDialogState.release = release
    updateDialogState.isOpen = true
  }
}