<script lang="ts">
    import { ChevronDown, ChevronRight, File, Folder, Terminal as TerminalIcon } from "lucide-svelte";
    import type { FileEntry } from "$lib/utils/types";
    import { createEventDispatcher } from "svelte";
    import { loadFiles as loadFilesUtil, updateAllFiles as flattenFilesUtil } from "$lib/utils/fileLoader";

    const dispatch = createEventDispatcher();

    // Props from parent
    export let files: FileEntry[] = [];
    export let allFiles: FileEntry[] = [];
    export let selectedFilePath: string | null = null;
    export let projectPath: string | null = null;
    export let sidebarWidth: number = 300;
    export let isSidebarOpen: boolean = true;
    export let isTerminalOpen: boolean = false;
    export let terminalHeight: number = 200;

    function handleResize(event: MouseEvent) {
        const startX = event.clientX;
        const startWidth = sidebarWidth;
        const maxWidth = 600;

        function onMouseMove(moveEvent: MouseEvent) {
            const newWidth = startWidth + (moveEvent.clientX - startX);
            const clamped = Math.max(100, Math.min(maxWidth, newWidth));
            sidebarWidth = clamped;
            dispatch("resize", { width: clamped });
        }

        function onMouseUp() {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        }

        window.addEventListener("mousemove", onMouseMove);
        window.addEventListener("mouseup", onMouseUp);
    }

    function toggleSidebar() {
        const newWidth = isSidebarOpen ? 0 : 300;
        dispatch("toggleSidebar", { width: newWidth });
    }

    function toggleTerminal() {
        dispatch("toggleTerminal");
    }

    async function onSelectFile(file: FileEntry, event: MouseEvent) {
        event.preventDefault();
        if (event.button === 0) {
            // Left click
            if (file.is_dir) {
                file.expanded = !file.expanded;
                if (file.expanded && (!file.children || file.children.length === 0)) {
                    try {
                        file.children = await loadFilesUtil(file.path, (file.level || 0) + 1);
                    } catch (e) {
                        console.error('Failed to load children for', file.path, e);
                    }
                }
                // Recompute flattened list and notify parent to sync store
                allFiles = flattenFilesUtil(files);
                dispatch('filesChanged', { files });
                return;
            } else {
                // Open file
                dispatch('openFile', { file });
            }
        } else if (event.button === 2) {
            // Right click -> forward to parent to open context menu
            dispatch('contextMenu', { mouseEvent: event, isDir: file.is_dir, path: file.path, projectPath });
        }
    }
</script>

<div class="sidebar--tools">
    <button
        class="sidebar--tools-item"
        class:active={isSidebarOpen}
        on:click={toggleSidebar}
        title="Toggle Sidebar"
    >
        <Folder size={25} />
    </button>

    <button
        class="sidebar--tools-item bottom"
        class:active={isTerminalOpen}
        on:click={toggleTerminal}
        title="Toggle Terminal"
    >
        <TerminalIcon size={25} />
    </button>
</div>

<div class="sidebar" style="width: {sidebarWidth}px;">
    {#if projectPath}
        <div
            class="file-list"
            style="height: {isTerminalOpen ? `calc(100vh - ${terminalHeight}px)` : 'calc(100vh - 25px)'};"
        >
            {#if allFiles.length > 0}
                <ul>
                    {#each allFiles as file}
                        <li>
                            <button
                                on:mousedown={(event) => onSelectFile(file, event)}
                                class={`file-list-item ${selectedFilePath === file.path ? 'selected' : ''} ${file.is_dir ? 'directory' : 'file'}`}
                                style={`padding-left: ${(file.level || 0) * 1.5 + 0.5}rem`}
                            >
                                <span class="item-icon">
                                    {#if file.is_dir}
                                        {#if file.expanded}
                                            <ChevronDown size={16} />
                                        {:else}
                                            <ChevronRight size={16} />
                                        {/if}
                                        <Folder size={16} />
                                    {:else}
                                        <File size={16} />
                                    {/if}
                                </span>
                                {file.name}
                            </button>
                        </li>
                    {/each}
                </ul>
            {:else}
                <p>No files found</p>
            {/if}
        </div>
    {:else}
        <p>No project opened</p>
    {/if}
</div>

<button aria-label="resizer" class="resizer" on:mousedown={handleResize}></button>

<style lang="scss">
  @use "../style/sidebar.scss";
</style>
