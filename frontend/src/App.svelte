<script lang="ts">
    import "./app.css";
    import aacLogo from "./assets/images/aac-logo-wide.png";
    /* import daru from "./assets/images/doodleru.png";
    import Navbar_Old from "./components/Navbar_Old.svelte"; */
    import { onDestroy, onMount } from "svelte";
    import addons from "./addons";
    import UI from "./screens/UI.svelte";

    onMount(async () => {
        await addons.populateAddonStore();
    });

    // TODO: Add a way to reset addon settings file because it can contain parsing errors
</script>

<!-- <div class="container-flex-row">
    <aside class="app-navbar-wrap" id="NavBarMain" style="">
        <nav class="app-navbar" style="padding-top: 0.5rem">
            <div class="aac-logo-container">
                <img
                    src={aacLogo}
                    style="width: 70%"
                    alt="ArcheAge Classic Logo"
                />
            </div>

            <div class="app-flex app-flex-center" style="margin-top: 1rem">
                <img
                    src={daru}
                    style="width: 70%"
                    alt="ArcheAge Classic Logo"
                />
            </div>

            <Navbar_Old />
        </nav>
    </aside>
    <div class="app-page-container px-md">
        <ActiveScreen />
    </div>
</div -->

<UI />
