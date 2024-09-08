<script lang="ts">
    export let header = '';
    let div = null;

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
            <slot name="body"></slot>
        </div>
        <div class="app-dialog-footer">
            <slot name="footer">
                <button class="app-btn" type="button" on:click={toggle}>
                    <span>Close</span>
                </button>
            </slot>
        </div>
    </div>
</div>