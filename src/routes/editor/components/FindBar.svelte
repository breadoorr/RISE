<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let visible: boolean = false;
  export let query: string = '';
  export let replacement: string = '';
  export let caseSensitive: boolean = false;
  export let count: number = 0; // total matches
  export let index: number = 0; // 0-based current match index

  const dispatch = createEventDispatcher();

  let findInput: HTMLInputElement;

  // Expose focus method to parent via bind:this if desired
  export function focusFind() {
    if (findInput) {
      findInput.focus();
      findInput.select();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') { dispatch('close'); e.stopPropagation(); }
    else if (e.key === 'Enter') { if (e.shiftKey) { dispatch('prev'); } else { dispatch('next'); } e.preventDefault(); }
  }

  onMount(() => {
    if (visible) {
      setTimeout(() => focusFind(), 0);
    }
  });
</script>

{#if visible}
  <div class="find-bar" role="toolbar" tabindex="0" aria-label="Find and replace" on:keydown={handleKeyDown}>
    <input class="find-input" bind:this={findInput} type="text" placeholder="Find" bind:value={query} on:input={() => dispatch('updateQuery', { value: query })} />
    <input class="replace-input" type="text" placeholder="Replace" bind:value={replacement} on:input={() => dispatch('updateReplacement', { value: replacement })} />
    <label class="case-toggle" title="Case sensitive">
      <input type="checkbox" bind:checked={caseSensitive} on:change={() => dispatch('updateCase', { value: caseSensitive })} /> Aa
    </label>
    <button title="Previous match" on:click={() => dispatch('prev')}>↑</button>
    <button title="Next match" on:click={() => dispatch('next')}>↓</button>
    <button title="Replace" on:click={() => dispatch('replaceOne')}>Replace</button>
    <button title="Replace all" on:click={() => dispatch('replaceAll')}>All</button>
    <span class="count">{count > 0 ? `${index + 1}/${count}` : '0/0'}</span>
    <button class="close" title="Close" on:click={() => dispatch('close')}>×</button>
  </div>
{/if}

<style lang="scss">
  .find-bar {
    width: calc(100% - 14px);
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    background: var(--background-50);
    color: var(--text-700);
    padding: 6px 8px;
    border-radius: 8px;
    z-index: 5;
    border: 1px solid var(--background-100);
    backdrop-filter: blur(2px);
  }
  .find-bar input {
    background: var(--secondary-100);
    color: var(--text-800);
    border: 1px solid var(--secondary-300);
    padding: 6px 8px;
    border-radius: 6px;
    outline: none;
    flex: 1 1 140px;
  }
  /* Prefer a bit more space for the main find field */
  .find-bar input.find-input { flex: 2 1 200px; min-width: 160px; }
  .find-bar input:focus {
    border-color: var(--accent-500);
  }
  .find-bar button {
    background: var(--secondary-100);
    color: var(--text-800);
    border: 1px solid var(--secondary-200);
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
    flex: 0 0 auto; /* keep natural size */
  }
  .find-bar button:hover { background: var(--secondary-200); border-color: var(--secondary-300); }
  .find-bar .close { font-weight: bold; padding: 2px 8px; background: transparent; border: none; font-size: 16px; }
  .find-bar .close:hover { color: var(--text-900); }
  .find-bar .count { opacity: 0.9; font-size: 12px; margin-left: 6px; font-variant-numeric: tabular-nums; flex: 0 0 auto; }
  .find-bar .case-toggle { display: inline-flex; align-items: center; gap: 4px; padding: 0 4px; user-select: none; flex: 0 0 auto; }
  .find-bar .case-toggle input { width: 14px; height: 14px; }
</style>
