// import { writable } from "svelte/store";
// import AddonsScreen from "../screens/AddonsScreen.svelte";
import DashboardScreen from "../screens/DashboardScreen.svelte";

// export const activeScreen = writable(null);
let activeScreen = $state(DashboardScreen);
export function getActiveScreen() {
    return activeScreen;
}
export function setActiveScreen(screen: any) {
    activeScreen = screen;
}