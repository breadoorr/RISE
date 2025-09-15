<script lang="ts">
    let Actions: Array<String> = $state([]);

    let isMenuOpen = $state(false);

    let x = $state(10);
    let y = $state(10);

    export function toggleFileMenu(e: MouseEvent, actions: Array<String>) {
        isMenuOpen = !isMenuOpen;

        if (isMenuOpen) {
            x = e.clientX;
            y = e.clientY;

            Actions = actions;
        }
    }
</script>

<div class="file-menu-container" style="display: {isMenuOpen ? 'flex' : 'none'}; left: {x}px; top: {y}px">
    {#if Actions.length > 0}
        {#each Actions as action}
            <button class="file-menu-item">{action}</button>
        {/each}
    {/if}
</div>

<style lang="scss">
    @use 'file-menu';
</style>