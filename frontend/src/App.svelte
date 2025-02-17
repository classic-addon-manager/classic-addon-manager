<script lang="ts">
    import "./app.css";
    import {Toaster} from "$lib/components/ui/sonner/index.js";
    import {onMount} from "svelte";
    import addons from "./addons";
    import UI from "./screens/UI.svelte";

    onMount(async () => {
        await addons.populateAddonStore();
    });
</script>

<Toaster closeButton/>
<UI/>
