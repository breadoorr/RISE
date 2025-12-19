<script lang="ts">
    export let Actions: string[] = [];
    export let isMenuOpen = false;
    export let x = 0;
    export let y = 0;

    export let triggerAction = (action: string) => Promise<void>;
</script>

{#if Actions.length > 0}
<div class="menu-container" style="display: {isMenuOpen ? 'flex' : 'none'}; left: {x}px; top: {y}px">
        {#each Actions as action}
            <button on:click={()=>triggerAction(action)} class="menu-item">{action}</button>
        {/each}
</div>
{/if}

<style lang="scss">
  @use '../style/menu';
</style>