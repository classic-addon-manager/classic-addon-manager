import type {Component} from 'svelte'

let screen: Component | null = $state(null)

async function loadDefaultScreen() {
  const {default: DashboardScreen} = await import('$screens/DashboardScreen.svelte')
  if (screen === null) {
    screen = DashboardScreen
  }
}

loadDefaultScreen().catch((err) => console.error(err))

export function getActiveScreen(): Component | null {
  return screen
}

export function setActiveScreen(newScreen: Component): void {
  screen = newScreen
}
