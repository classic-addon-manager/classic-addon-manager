import { create } from 'zustand'

import { ApplicationService } from '@/lib/wails'

interface User {
  username: string
  avatar: string
  discord_id: string
}

interface UserState {
  // State
  user: User
  token: string

  // Actions
  isAuthenticated: () => boolean
  setUser: (newUser: User) => void
  setToken: (token: string) => void
  clearUser: () => void
}

const initialState = {
  user: {
    username: '',
    avatar: '',
    discord_id: '',
  },
  token: localStorage.getItem('token') || '',
}

// Initialize the auth token in the application service if it exists
if (initialState.token) {
  ApplicationService.SetAuthToken(initialState.token)
}

export const useUserStore = create<UserState>((set, get) => ({
  ...initialState,

  // Actions
  isAuthenticated: () => {
    const { user } = get()
    return user.discord_id !== ''
  },

  setUser: (newUser: User) => {
    // Ensure we never set an invalid user object
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

  setToken: (token: string) => {
    set({ token })
    localStorage.setItem('token', token)
    ApplicationService.SetAuthToken(token)
  },

  clearUser: () => {
    set({ ...initialState })
    localStorage.removeItem('token')
    ApplicationService.SetAuthToken('')
  },
}))
