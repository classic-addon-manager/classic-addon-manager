import { AlertTriangleIcon, CheckIcon } from 'lucide-react'
import { create } from 'zustand'

import { toast } from '@/components/ui/toast'
import { apiClient } from '@/lib/api'
import type { AddonManifest } from '@/lib/wails'
import { ApplicationService, LocalAddonService, RemoteAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

interface User {
  username: string
  avatar: string
  discord_id: string
}

interface UserState {
  user: User
  token: string
  authBootstrapComplete: boolean

  isAuthenticated: () => boolean
  setUser: (newUser: User) => void
  saveToken: (token: string) => Promise<void>
  fetchCurrentUser: () => Promise<void>
  bootstrapAuth: () => Promise<void>
  signOut: () => Promise<void>
  clearLocalAuthState: () => void
}

const emptyUser: User = {
  username: '',
  avatar: '',
  discord_id: '',
}

let restoreInFlight: Promise<void> | null = null

function restoreSubscribedAddons() {
  if (!restoreInFlight) {
    restoreInFlight = runRestore().finally(() => {
      restoreInFlight = null
    })
  }

  return restoreInFlight
}

async function runRestore() {
  let addons: AddonManifest[]

  try {
    addons = await RemoteAddonService.GetSubscribedAddons()
  } catch (error) {
    console.error('Error fetching subscribed addons:', error)
    toast({
      title: 'Error',
      description: 'Failed to fetch subscribed addons.',
      icon: AlertTriangleIcon,
    })
    return
  }

  if (addons.length === 0) return

  let installed = 0
  let failed = 0
  let stateChanged = false

  for (const addon of addons) {
    try {
      if (await LocalAddonService.IsInstalled(addon.name)) continue

      const result = await RemoteAddonService.InstallAddonWithDependencies(addon, 'latest')
      stateChanged = true

      if (result.success) {
        installed++
      } else {
        console.error(`Install reported failure for ${addon.name}:`, result.mainAddon?.error)
        failed++
      }
    } catch (error) {
      console.error(`Error restoring ${addon.name}:`, error)
      failed++
    }
  }

  if (stateChanged) {
    await useAddonStore.getState().refreshAfterAddonChange()
  }

  if (installed > 0) {
    toast({
      title: 'Addons restored',
      description: `${installed} addon(s) were restored from the server.`,
      icon: CheckIcon,
    })
  }

  if (failed > 0) {
    toast({
      title: 'Error',
      description: `${failed} addon(s) failed to restore. Check logfile for details.`,
      icon: AlertTriangleIcon,
    })
  }
}

export const useUserStore = create<UserState>((set, get) => ({
  user: { ...emptyUser },
  token: '',
  authBootstrapComplete: false,

  isAuthenticated: () => {
    const { user } = get()
    return user.discord_id !== ''
  },

  setUser: (newUser: User) => {
    if (!newUser) {
      console.error('Attempted to set invalid user object:', newUser)
      return
    }
    set({
      user: {
        username: newUser.username ?? '',
        avatar: newUser.avatar ?? '',
        discord_id: newUser.discord_id ?? '',
      },
    })
  },

  saveToken: async (token: string) => {
    if (!token || typeof token !== 'string') {
      throw new Error('Invalid token')
    }
    await ApplicationService.SaveAuthToken(token)
    set({ token })
    await get().fetchCurrentUser()
  },

  fetchCurrentUser: async () => {
    const currentToken = get().token
    if (!currentToken) return

    try {
      const resp = await apiClient.get('/me')
      if (resp.status === 200) {
        const userData = await resp.json()
        get().setUser(userData)
        void restoreSubscribedAddons()
      } else if (resp.status === 401) {
        await ApplicationService.ClearAuthToken()
        get().clearLocalAuthState()
      }
    } catch (error) {
      console.error('Error fetching user data:', error)
    }
  },

  bootstrapAuth: async () => {
    try {
      localStorage.removeItem('token')
    } catch {
      // ignore
    }

    try {
      const session = await ApplicationService.GetAuthSession()
      if (session?.token) {
        set({ token: session.token })
        await get().fetchCurrentUser()
      }
    } catch (error) {
      console.error('Auth bootstrap failed:', error)
    } finally {
      set({ authBootstrapComplete: true })
    }
  },

  signOut: async () => {
    try {
      await ApplicationService.ClearAuthToken()
      get().clearLocalAuthState()
    } catch (error) {
      console.error('Sign out failed:', error)
      throw error
    }
  },

  clearLocalAuthState: () => {
    set({ token: '', user: { ...emptyUser } })
  },
}))
