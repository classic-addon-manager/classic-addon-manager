<script lang="ts">
    import NavbarItem from "./NavbarItem.svelte";

    import Link from "lucide-svelte/icons/link";
    import {BrowserOpenURL} from "$lib/wails";
</script>

<a
        href="##"
        onclick={() => BrowserOpenURL("https://aa-classic.com")}
>
    <NavbarItem name="AA Classic Website" Icon={Link} isActivatable={false}/>
</a>
