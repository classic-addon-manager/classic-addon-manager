import { create } from 'zustand'

import { apiClient } from '@/lib/api'
import { ApplicationService } from '@/lib/wails'

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
