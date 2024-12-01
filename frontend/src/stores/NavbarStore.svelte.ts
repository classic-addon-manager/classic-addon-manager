let activeNavbar: string = $state('dashboard');

export function getActiveNavbar() {
    return activeNavbar;
}

export function setActiveNavbar(navbar: string) {
    activeNavbar = navbar;
}