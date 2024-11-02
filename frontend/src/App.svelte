<script lang="ts">
    import aacLogo from './assets/images/aac-logo-wide.png';
    import daru from './assets/images/doodleru.png';
    import Navbar from "./components/Navbar.svelte";
    import {activeScreen} from "./stores/ScreenStore";
    import {onDestroy, onMount} from "svelte";
    import addons from "./addons";

    let ActiveScreen: any = $state();
    let unsub = activeScreen.subscribe(value => {
        ActiveScreen = value;
    });

    onMount(async () => {
        await addons.populateAddonStore();
    });

    // TODO: Add a way to reset addon settings file because it can contain parsing errors
    onDestroy(() => {
        unsub();
    });
</script>


<div class="container-flex-row">
    <aside class="app-navbar-wrap" id="NavBarMain" style="">
        <nav class="app-navbar" style="padding-top: 0.5rem">
            <div class="aac-logo-container">
                <img src={aacLogo} style="width: 70%" alt="ArcheAge Classic Logo">
            </div>

            <div class="app-flex app-flex-center" style="margin-top: 1rem">
                <img src={daru} style="width: 70%" alt="ArcheAge Classic Logo">
            </div>


            <Navbar />
        </nav>
    </aside>
    <div class="app-page-container px-md">
        <ActiveScreen />
    </div>
</div>

<style>
    .aac-logo-container {
        display: flex;
        justify-content: center;
        align-items: center;
    }
</style>