import {writable} from "svelte/store";
import AddonsScreen from "../screens/AddonsScreen.svelte";

export const activeScreen = writable(AddonsScreen)