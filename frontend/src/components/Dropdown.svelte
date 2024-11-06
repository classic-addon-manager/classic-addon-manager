<script lang="ts">
    interface Props {
        options: {label: string, value: string}[];
        valueChanged: (value: string) => void;
    }
    
    let { options,  valueChanged }: Props = $props();
</script>

<div class="app-select-menu">
    <select onchange={(event: Event) => {
        valueChanged((event.target as HTMLSelectElement).value);
    }}>
    {#each options as option}
        <option value={option.value}>{option.label}</option>
    {/each}
</select>
</div>