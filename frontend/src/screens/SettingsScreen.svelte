<script lang="ts">
    import {onMount} from "svelte";
    import {GetConfigString, SelectDirectory, SetConfigString} from "../../wailsjs/go/main/App";

    let aacPath = '';

    onMount(async () => {
        aacPath = await GetConfigString('general.aacpath');
    });


</script>

<main style="width: 100%">
    <h1>Settings</h1>

    <label for="aac-path">ArcheAge Classic Path:</label>
    <div class="app-flex" style="flex-flow: row">
        <div class="app-input-container" style="flex: 1">
            <input
                    on:click={async () => {
                    let path = await SelectDirectory();
                    if(path) {
                        aacPath = path;
                        await SetConfigString('general.aacpath', path);
                    }
                }}
                    class="app-input-text" style="width: 100%" id="aac-path" type="text" placeholder="Enter ArcheAge Classic path" autocomplete="off" value={aacPath}>
        </div>
        <button class="app-btn" style="flex-shrink: 1; flex-grow: 0.5; margin-left: 0.5em"
            on:click={async () => {
                let path = await SelectDirectory();
                if(path) {
                    aacPath = path;
                    await SetConfigString('general.aacpath', path);
                }
            }}
        >
            <span>Browse</span>
        </button>
    </div>

    <br/>
</main>

<style>
</style>