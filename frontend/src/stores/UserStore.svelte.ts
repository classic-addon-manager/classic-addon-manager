import {ApplicationService} from "$lib/wails";

export interface User {
    username: string;
    avatar: string;
    discord_id: string;
}

interface UserState {
    user: User;
    token: string;
}

const initialState: UserState = {
    user: {
        username: "",
        avatar: "",
        discord_id: ""
    },
    token: localStorage.getItem('token') || ""
};

let state = $state<UserState>(initialState);

// Initialize token in ApplicationService if it exists
if (state.token) {
    ApplicationService.SetAuthToken(state.token);
}

// Exported functions that return the current state values
export function isAuthenticated(): boolean {
    return state.user.discord_id !== "";
}

export function getUser(): User {
    return state.user;
}

export function getToken(): string {
    return state.token;
}

// Actions
export function setUser(newUser: User) {
    state.user = newUser;
}

export function setToken(newToken: string) {
    state.token = newToken;
    localStorage.setItem('token', newToken);
    ApplicationService.SetAuthToken(newToken);
}

// Clear user state (e.g. logout)
export function clearUserState() {
    state = initialState;
    localStorage.removeItem('token');
    ApplicationService.SetAuthToken("");
}