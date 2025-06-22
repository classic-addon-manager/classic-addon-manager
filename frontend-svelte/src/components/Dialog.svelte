<script lang="ts">
    interface Props {
        header?: string;
        body?: import('svelte').Snippet;
        footer?: import('svelte').Snippet;
    }

    let { header = '', body, footer }: Props = $props();
    let div: HTMLDivElement|null = $state(null);

    export function toggle() {
        if(div === null) return;
        if(div.classList.contains("show")) {
            div.classList.remove("show");
        } else {
            div.classList.add("show");
        }
    }
</script>

<div bind:this={div} class="app-dialog"  data-win-toggle="modal" tabindex="-1">
    <div class="app-dialog-modal" aria-modal="true" role="dialog">
        <div class="app-dialog-header">
            <h3>{header}</h3>
        </div>
        <div class="app-dialog-body">
            {@render body?.()}
        </div>
        <div class="app-dialog-footer">
            {#if footer}{@render footer()}{:else}
                <button class="app-btn" type="button" onclick={toggle}>
                    <span>Close</span>
                </button>
            {/if}
        </div>
    </div>
</div>