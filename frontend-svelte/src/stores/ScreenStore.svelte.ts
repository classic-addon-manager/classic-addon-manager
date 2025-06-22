import DashboardScreen from "../screens/DashboardScreen.svelte";

let activeScreen = $state(DashboardScreen);

export function getActiveScreen() {
    return activeScreen;
}

export function setActiveScreen(screen: any) {
    activeScreen = screen;
}