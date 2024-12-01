<script lang="ts">
    import NavbarItem from "./NavbarItem.svelte";

    import Link from "lucide-svelte/icons/link";
</script>

<a
    href="##"
    onclick={() => {
        // @ts-ignore
        window.runtime.BrowserOpenURL("https://aa-classic.com");
    }}
>
    <NavbarItem name="AA Classic Website" Icon={Link} isActivatable={false} />
</a>
