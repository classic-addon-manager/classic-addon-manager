let activeNavbar: string = $state('dashboard')

export function getActiveNavbar(): string {
  return activeNavbar
}

export function setActiveNavbar(newActiveItem: string): void {
  activeNavbar = newActiveItem
}