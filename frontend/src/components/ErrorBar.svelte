<script lang="ts">
    interface Props {
        message: string;
    }

    let { message = $bindable() }: Props = $props();
</script>

<div class="app-alert-bar alert-bar-danger" role="alert">
    <i class="icons10-cross app-color-danger"></i>
    <strong class="app-color-danger">Error</strong><span>{message}</span>
    <div class="app-alert-bar-end-content">
        <button data-win-dismiss="alert" aria-label="Close" onclick={() => message = ''}></button>
    </div>
</div>