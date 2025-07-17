import {ApplicationService} from '../../bindings/ClassicAddonManager/backend/services'

export interface User {
    username: string
    avatar: string
    discord_id: string
}

interface UserState {
    user: User
    token: string
}

const initialState: UserState = {
  user: {
    username: '',
    avatar: '',
    discord_id: ''
  },
  token: localStorage.getItem('token') || ''
}

let state = $state<UserState>(initialState)

// Initialize token in ApplicationService if it exists
if (state.token) {
  ApplicationService.SetAuthToken(state.token)
}

export function isAuthenticated(): boolean {
  return state.user.discord_id !== ''
}

export function getUser(): User {
  return state.user
}

export function getToken(): string {
  return state.token
}

export function setUser(newUser: User): void {
  state.user = newUser
}

export function setToken(newToken: string): void {
  state.token = newToken
  localStorage.setItem('token', newToken)
  ApplicationService.SetAuthToken(newToken)
}

// Clear user state (e.g. logout)
export function clearUserState(): void {
  state = initialState
  localStorage.removeItem('token')
  ApplicationService.SetAuthToken('')
}