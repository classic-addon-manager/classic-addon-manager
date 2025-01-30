export type User = {
    username: string,
    avatar: string,
    discord_id: string
}

let user = $state<User>({
    username: "",
    avatar: "",
    discord_id: ""
});

let token = $state<string>("");

export function setUser(newUser: User) {
    user = newUser;
}

export function getUser(): User {
    return user;
}

export function isAuthenticated(): boolean {
    return user.discord_id !== "";
}

export function getToken(): string {
    if (token === "") {
        token = localStorage.getItem('token') || "";
    }
    return token;
}

export function setToken(newToken: string) {
    token = newToken;
    localStorage.setItem('token', token);
}