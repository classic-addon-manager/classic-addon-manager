<script lang="ts">
    import {activeScreen} from "../stores/ScreenStore";
    import AddonsScreen from "../screens/AddonsScreen.svelte";
    import DiscoverAddonsScreen from "../screens/DiscoverAddonsScreen.svelte";
    import packageJson from '../../package.json';

    let active = $state('local_addons');
</script>

<div class="fill-parent app-flex" style="align-items: flex-end">
    <div style="width: 100%">
        <ul class="app-navbar-list" id="app-navbar-list" style="flex-flow: column">
            <li class="app-navbar-list-item">
                <!-- TODO: Show button to download update if there is one -->
                <a href="#" style="font-weight: normal; color: #676767">
                    {packageJson.version} (Beta) by Sami
                </a>
            </li>
            <!-- Move this to settings -->
            <!--<label class="app-navbar-theme-switch">
                <input id="app-navbar-theme-switch" type="checkbox">
                <div class="app-navbar-theme-switch-icon"></div>
            </label>-->

            <li class="app-navbar-list-item">
                <a class:active={active === 'local_addons'} href="#" onclick={() => {
                    $activeScreen = AddonsScreen
                    active = 'local_addons'
                }}>
                    <i class="icons10-hdd"></i>
                    <span>Installed Addons</span>
                </a>
            </li>

            <li class="app-navbar-list-item">
                <a class:active={active === 'discover_addons'} href="#" onclick={() => {
                    $activeScreen = DiscoverAddonsScreen
                    active = 'discover_addons'
                }}>
                    <i class="icons10-download"></i>
                    <span>Get More Addons</span>
                </a>
            </li>

            <!--<li class="app-navbar-list-item"><a href="#">
                <i class="icons10-exclamation-mark"></i>
                <span>Update Available</span>
            </a></li>-->

            <!--<li class="app-navbar-list-item"><a href="#" on:click={() => activeScreen.set(SettingsScreen)}>
                <i class="icons10-settings"></i>
                <span>Settings</span>
            </a></li>-->

            <li class="app-navbar-list-item"><a href="https://aa-classic.com" target="_blank">
                <i class="icons10-link"></i>
                <span>ArcheAge Classic Website</span>
            </a></li>

            <li class="app-navbar-list-item"><a href="#" class="undefined" onclick={() => {
                // @ts-ignore
                window.runtime.Quit()
            }}>
                <i class="icons10-cross"></i>
                <span>Exit</span>
            </a></li>
        </ul>

    </div>
</div>
