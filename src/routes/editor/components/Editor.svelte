<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/tauri";
  import type { FileEntry } from "$lib/utils/types";

  // Props from parent
  export let sidebarWidth: number = 300;
  export let isTerminalOpen: boolean = false;
  export let terminalHeight: number = 200;

  export let openFiles: FileEntry[] = [];
  export let activeFileIndex: number = -1;

  export let selectedFile: string | null = null;
  export let fileContent: string = "";
  export let editorContent: string = "";
  export let lineCount: number = 1;
  export let currentLine: number = 1;
  export let currentColumn: number = 1;
  export let highlightHtml: string = "";
  export let isEdited: boolean = false;

  // Tab callbacks remain (tab switching/closing still handled by parent for minimal changes)
  export let onTabClick: (index: number) => void = () => {};
  export let onTabClose: (index: number, event: MouseEvent) => void = () => {};

  // Provide internal refs up to parent if needed
  let editorElement: HTMLTextAreaElement;
  let editorWrapper: HTMLDivElement;
  let highlightContainer: HTMLDivElement;
  export let setEditorRefs: (refs: { editorElement: HTMLTextAreaElement; editorWrapper: HTMLDivElement; highlightContainer: HTMLDivElement }) => void = () => {};

  $: setEditorRefs && setEditorRefs({ editorElement, editorWrapper, highlightContainer });

  function getDisplayName(file: FileEntry, allOpen: FileEntry[]): string {
    const sameNameCount = allOpen.filter((f) => f.name === file.name).length;
    if (sameNameCount > 1 && file.parent_dir) {
      return `${file.parent_dir}/${file.name}`;
    }
    return file.name;
  }

  // Internal editor state
  let lastBufferContent: string = "";
  let autoSaveTimeout: number | null = null;
  let highlightTimeout: number | null = null;

  // Lifecycle: manage key/focus/click listeners related to editor behavior
  onMount(() => {
    const handleKeyDown = async (event: KeyboardEvent) => {
      if (event.key === 'Tab' && event.target === editorElement) {
        event.preventDefault();
        const target = event.target as HTMLTextAreaElement;
        const start = target.selectionStart;
        const end = target.selectionEnd;
        const text = target.value;
        const before = text.slice(0, start);
        const after = text.slice(end);
        const indent = '    ';
        const result = before + indent + after;
        target.selectionStart = start + indent.length;
        lastBufferContent = result;
        editorContent = result;
        isEdited = result !== fileContent;
        if (editorElement) editorElement.value = result;
        await updateLineNumbers(result);
        scheduleHighlight();
      }
      if (event.ctrlKey || event.metaKey) {
        if (event.key === 's') {
          event.preventDefault();
          if (selectedFile && isEdited) {
            await saveFile();
          }
        } else if (event.key === 'z') {
          event.preventDefault();
          if (!selectedFile) return;
          const result = await invoke('undo_last_change', { path: selectedFile }) as string;
          editorContent = result;
          lastBufferContent = result;
          isEdited = result !== fileContent;
          if (editorElement) editorElement.value = result;
          await updateLineNumbers(result);
          scheduleHighlight();
        }
      }
    };

    const handleInputEvent = (event: Event) => {
      if (event.target === editorElement && editorElement.value !== undefined) {
        updateCurrentLine(event);
      }
    };

    const restoreEditorContent = () => {
      if (selectedFile && activeFileIndex >= 0 && editorElement) {
        editorElement.value = editorContent;
        updateLineNumbers(editorContent);
        currentLine = editorContent.slice(0, editorElement.selectionStart).split('\n').length;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleInputEvent);
    window.addEventListener('click', handleInputEvent);
    window.addEventListener('focus', restoreEditorContent);

    // Initial sync
    syncLineNumbersScroll();

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleInputEvent);
      window.removeEventListener('click', handleInputEvent);
      window.removeEventListener('focus', restoreEditorContent);
      if (autoSaveTimeout !== null) clearTimeout(autoSaveTimeout);
      if (highlightTimeout !== null) clearTimeout(highlightTimeout);
    };
  });

  function updateCurrentLine(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    if (target === editorElement && target.value !== undefined) {
      editorContent = target.value;
      currentLine = editorContent.slice(0, target.selectionStart).split('\n').length;
      currentColumn = target.selectionStart - editorContent.slice(0, target.selectionStart).lastIndexOf('\n');
    }
  }

  async function handleEditorChange(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    if (target === editorElement && target.value !== undefined) {
      const newVal = target.value;
      if (!selectedFile || fileContent === "Cannot display contents of the file") {
        editorContent = newVal;
        await updateLineNumbers(editorContent);
        updateCurrentLine(event);
        scheduleHighlight();
        return;
      }
      try {
        const updated = await invoke('apply_full_update', { path: selectedFile, newContent: newVal }) as string;
        editorContent = updated;
        lastBufferContent = updated;
        isEdited = updated !== fileContent;
        if (editorElement) editorElement.value = updated;
        await updateLineNumbers(updated);
        updateCurrentLine(event);
        scheduleHighlight();
        if (isEdited && selectedFile) {
          if (autoSaveTimeout !== null) clearTimeout(autoSaveTimeout);
          autoSaveTimeout = setTimeout(autoSave, 500);
        }
      } catch (e) {
        console.error('apply_full_update failed', e);
      }
    }
  }

  async function autoSave() {
    if (isEdited && selectedFile && fileContent !== "Cannot display contents of the file") {
      try {
        const buf = await invoke('get_buffer', { path: selectedFile }) as string;
        editorContent = buf;
        await invoke("write_file", { path: selectedFile, content: buf });
        fileContent = buf;
        isEdited = false;
        console.log("Auto-saved file successfully");
      } catch (error) {
        console.error("Error auto-saving file:", error);
      }
    }
  }

  async function updateLineNumbers(text: string) {
    try {
      if (selectedFile && fileContent !== "Cannot display contents of the file") {
        const count = await invoke('get_line_count', { path: selectedFile }) as number;
        lineCount = Math.max(1, count || 1);
      } else {
        lineCount = Math.max(1, (text ? text.split('\n').length : 1));
      }
    } catch (e) {
      console.error('get_line_count failed, falling back:', e);
      lineCount = Math.max(1, (text ? text.split('\n').length : 1));
    }
    syncLineNumbersScroll();
  }

  function detectLanguageFromFilename(file: string | null): string {
    if (!file) return 'typescript';
    const f = file.toLowerCase();
    if (f.endsWith('.rs')) return 'rust';
    if (f.endsWith('.py')) return 'python';
    if (f.endsWith('.c') || f.endsWith('.h')) return 'c';
    if (f.endsWith('.java')) return 'java';
    if (f.endsWith('.cs')) return 'c_sharp';
    if (f.endsWith('.sql')) return 'sequel';
    if (f.endsWith('.ts') || f.endsWith('.tsx') || f.endsWith('.js') || f.endsWith('.jsx')) return 'typescript';
    return 'typescript';
  }

  function scheduleHighlight() {
    if (!selectedFile || fileContent === "Cannot display contents of the file") {
      highlightHtml = '';
      return;
    }
    if (highlightTimeout !== null) {
      clearTimeout(highlightTimeout);
    }
    const lang = detectLanguageFromFilename(selectedFile);
    const matches: number[] = [];
    const queryLen = 0;
    const path = selectedFile;
    highlightTimeout = setTimeout(async () => {
      try {
        highlightHtml = await invoke('highlight_html', { language: lang, matches, queryLen, path }) as string;
        syncLineNumbersScroll();
      } catch (e) {
        console.error('highlight error', e);
        highlightHtml = '';
      }
    }, 0);
  }

  function syncLineNumbersScroll() {
    if (editorWrapper && editorElement) {
      const lineNumbersContent = editorWrapper.querySelector('.line-numbers-content') as HTMLDivElement;
      if (lineNumbersContent) {
        lineNumbersContent.style.transform = `translateY(-${editorElement.scrollTop}px)`;
        lineNumbersContent.style.height = `${editorElement.scrollHeight}px`;
      }
      if (highlightContainer) {
        const x = editorElement.scrollLeft;
        const y = editorElement.scrollTop;
        highlightContainer.style.transform = `translate(${-x}px, ${-y}px)`;
        highlightContainer.style.height = `${editorElement.scrollHeight}px`;
        highlightContainer.style.width = `${editorElement.scrollWidth}px`;
      }
    }
  }

  async function saveFile() {
    if (!selectedFile) return;
    try {
      const buf = await invoke('get_buffer', { path: selectedFile }) as string;
      editorContent = buf;
      await invoke("write_file", { path: selectedFile, content: buf });
      fileContent = buf;
      isEdited = false;
    } catch (error) {
      console.error("Error saving file:", error);
      alert("Error saving file: " + error);
    }
  }

  // React: when selectedFile or content changes, ensure UI updates
  $: if (selectedFile !== null) {
    // re-run highlight when switching files
    scheduleHighlight();
  }
  $: (async () => { await updateLineNumbers(editorContent); })();
</script>

<div class="editor-area" style="width: calc(100% - {sidebarWidth}px - 5px); height: {isTerminalOpen ? `calc(100% - ${terminalHeight}px - 25px)` : 'calc(100vh - 25px)'};">
  <div class="editor-header" class:editor-header--closed={openFiles.length === 0}>
    {#each openFiles as file, index}
      <div class="file-tab" class:active={index === activeFileIndex}>
        <button class="file-tab--name-button" on:click={() => onTabClick(index)}>
          <span class="file-tab-name">{getDisplayName(file, openFiles)}</span>
        </button>
        <button class="file-tab-close" on:click={(e) => onTabClose(index, e)} title="Close file">×</button>
      </div>
    {/each}
  </div>

  {#if selectedFile}
    <div class="editor-wrapper" bind:this={editorWrapper} style="height: {isTerminalOpen ? `calc(100vh - ${terminalHeight}px - 70px)` : 'calc(100vh - 70px)'};">
      <div class="line-numbers">
        <div class="line-numbers-content">
          {#each Array(lineCount) as _, i}
            <div class="line-number" class:active={i + 1 === currentLine}>{i + 1}</div>
          {/each}
        </div>
      </div>
      <div class="code-editor--highlight" bind:this={highlightContainer}>
        {@html highlightHtml}
      </div>
      <textarea
        class={`code-editor ${fileContent === "Cannot display contents of the file" ? 'no-file-selected' : ''}`}
        class:with-syntax={highlightHtml && fileContent !== "Cannot display contents of the file"}
        bind:value={editorContent}
        bind:this={editorElement}
        on:input={handleEditorChange}
        on:scroll={syncLineNumbersScroll}
        spellcheck="false"
        disabled={fileContent === "Cannot display contents of the file"}
      ></textarea>
    </div>
  {:else}
    <div class="no-file-selected">
      <p>Select a file from the project explorer to edit</p>
    </div>
  {/if}
</div>
<style lang="scss">
    @use '../style/editor';
</style>