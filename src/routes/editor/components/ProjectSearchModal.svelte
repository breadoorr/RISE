<script lang="ts">
    import {createEventDispatcher} from 'svelte';

    export let open: boolean = false;
    export let busy: boolean = false;
    export let query: string = '';
    export let replacement: string = '';
    export let caseSensitive: boolean = false;
    export let results: { path: string; line: number; column: number; line_text: string }[] = [];

    const dispatch = createEventDispatcher();

    function onKeyDown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            dispatch('close');
            e.stopPropagation();
        }
    }

    function onQueryKeyDown(e: KeyboardEvent) {
        if (e.key === 'Enter') {
            dispatch('search');
        }
        if (e.key === 'Escape') {
            dispatch('close');
            e.stopPropagation();
        }
    }
</script>

{#if open}
    <div class="project-search-modal" on:keydown={onKeyDown}>
        <div class="modal-card">
            <div class="modal-header">
                <h3>Project Search</h3>
                <button class="close" on:click={() => dispatch('close')}>×</button>
            </div>
            <div class="modal-body">
                <div class="controls">
                    <input class="query" type="text" placeholder="Find text" bind:value={query}
                           on:keydown={onQueryKeyDown} on:input={() => dispatch('updateQuery', { value: query })}/>
                    <input class="replacement" type="text" placeholder="Replace with (optional)"
                           bind:value={replacement}
                           on:input={() => dispatch('updateReplacement', { value: replacement })}/>
                    <label><input type="checkbox" bind:checked={caseSensitive}
                                  on:change={() => dispatch('updateCase', { value: caseSensitive })}/>
                        Case-sensitive</label>
                    <button on:click={() => dispatch('search')} disabled={busy}>Search</button>
                    <button on:click={() => dispatch('preview')} disabled={busy || !query}>Preview</button>
                    <button on:click={() => dispatch('replace')} disabled={busy || !query}>Replace</button>
                </div>
                <div class="results">
                    {#if busy}
                        <div class="loading">Searching…</div>
                    {:else if results.length === 0}
                        <div class="empty">No results</div>
                    {:else}
                        <ul>
                            {#each results as r}
                                <li class="result" on:click={() => dispatch('openResult', r)}>
                                    <div class="path">{r.path}</div>
                                    <div class="loc">{r.line}:{r.column}</div>
                                    <div class="preview">{r.line_text}</div>
                                </li>
                            {/each}
                        </ul>
                    {/if}
                </div>
            </div>
        </div>
    </div>
{/if}

<style lang="scss">
  .project-search-modal {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    z-index: 50;
  }

  .modal-card {
    width: min(960px, 90vw);
    max-height: 80vh;
    background: var(--background-50);
    color: var(--text-800);
    border: 1px solid var(--secondary-200);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--secondary-200);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .modal-header .close {
    background: transparent;
    border: none;
    color: var(--text-800);
    font-size: 22px;
    cursor: pointer;
    line-height: 1;
  }

  .modal-header .close:hover {
    color: var(--text-900);
  }

  .modal-body {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .controls {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  input {
    background: var(--secondary-100);
    color: var(--text-800);
    border: 1px solid var(--secondary-200);
    padding: 8px 10px;
    border-radius: 8px;
  }

  input.query {
    min-width: 260px;
  }

  button {
    background: var(--secondary-100);
    color: var(--text-800);
    border: 1px solid var(--secondary-200);
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
  }

  button:hover {
    background: var(--secondary-200);
  }

  .results {
    overflow: auto;
    max-height: 60vh;
    border-top: 1px solid var(--secondary-200);
    padding: 10px 0  0 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .result {
    padding: 10px 8px;
    border-radius: 8px;
    border-bottom: 1px solid var(--secondary-200);
    cursor: pointer;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    grid-column-gap: 10px;
  }

  .result:hover {
    background: var(--secondary-100);
  }

  .result .path {
    grid-column: 1 / span 1;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    opacity: 0.9;
  }

  .result .loc {
    grid-column: 2 / span 1;
    font-size: 12px;
    opacity: 0.8;
    text-align: right;
  }

  .result .preview {
    grid-column: 1 / span 2;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.95;
  }
</style>
