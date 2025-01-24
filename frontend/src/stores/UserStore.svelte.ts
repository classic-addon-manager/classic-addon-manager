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

export function setUser(newUser: User) {
    user = newUser;
}

export function getUser(): User {
    return user;
}