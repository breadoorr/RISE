<script lang="ts">
    import { ChevronDown, ChevronRight, File, Folder, Terminal as TerminalIcon } from "lucide-svelte";
    import type { FileEntry } from "$lib/utils/types";
    import { createEventDispatcher } from "svelte";
    import { loadFiles as loadFilesUtil, updateAllFiles as flattenFilesUtil } from "$lib/utils/fileLoader";
    import FileMenu from "./FileMenu.svelte";
    import { fileStore } from "$lib/stores/fileStore";

    const dispatch = createEventDispatcher();
    export let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;
    let createNewItem: (isDir: boolean, parentPath: string, onNameConfirmed: (name: string) => Promise<void>) => void;

    // Props from parent
    export let files: FileEntry[] = [];
    export let allFiles: FileEntry[] = [];
    export let selectedFilePath: string | null = null;
    export let projectPath: string | null = null;
    export let sidebarWidth: number = 300;
    export let isSidebarOpen: boolean = true;
    export let isTerminalOpen: boolean = false;
    export let terminalHeight: number = 200;

    let editingItemPath: string | null;
    let newItemName: string = '';
    let currentOnNameConfirmed: ((name: string) => Promise<void>) | null = null;

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

    createNewItem = (isDir: boolean, parentPath: string | null, onNameConfirmed: (name: string) => Promise<void>) => {
        currentOnNameConfirmed = onNameConfirmed; // Store the callback for later use in saveNewItem
        const targetPath = parentPath || projectPath;
        const tempName = isDir ? "New Folder" : "New File";
        const tempPath = `${targetPath}/${tempName}`;

        const newItem: FileEntry = {
            name: tempName,
            path: tempPath,
            is_dir: isDir,
            expanded: false,
            children: isDir ? [] : undefined,
            level: parentPath ? (allFiles.find(f => f.path === parentPath)?.level || 0) + 1 : 0,
            isEditing: true,
            temp: true,
        };

        console.log("Creating new item:", newItem);

        allFiles = [...allFiles, newItem];
        editingItemPath = tempPath;
        newItemName = "";
    };

    async function saveNewItem(event: KeyboardEvent, item: FileEntry) {
        if (event.key === "Enter" && newItemName.trim() !== "") {
            try {
                // Update the item in the UI
                item.name = newItemName;
                item.path = `${item.path.split('/').slice(0, -1).join('/')}/${newItemName}`;
                item.isEditing = false;
                item.temp = false;

                // Call the stored callback with the final name
                if (currentOnNameConfirmed) {
                    await currentOnNameConfirmed(newItemName);
                }

                // Reset editing state
                editingItemPath = null;
                newItemName = "";
                currentOnNameConfirmed = null;

                console.log("Created item:", item);

                // Optionally open the file
                if (!item.is_dir) {
                    dispatch("openFile", { file: item });
                }

                // Recompute flattened list and notify parent to sync store
                allFiles = flattenFilesUtil(files);
                dispatch('filesChanged', { files });
            } catch (e) {
                console.error("Failed to create item:", e);
                allFiles = allFiles.filter(f => f.path !== item.path);
                currentOnNameConfirmed = null;
            }
        } else if (event.key === "Escape") {
            allFiles = allFiles.filter(f => f.path !== item.path);
            editingItemPath = null;
            newItemName = "";
            currentOnNameConfirmed = null;
        }
    }

    async function onSelectFile(file: FileEntry, event: MouseEvent) {
        event.preventDefault();
        if (event.button === 0) {
            toggleFileMenu(event, false);
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
                dispatch('openFile', { file });
            }
        } else if (event.button === 2) {
            event.preventDefault();
            toggleFileMenu(event, true, file.is_dir, file.path, projectPath);
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
            <FileMenu bind:toggleFileMenu bind:createNewItem />

            {#if allFiles.length > 0}
                <ul>
                    {#each allFiles as file}
                        <li>
                            {#if file.isEditing}
                                <div
                                        class="file-list-item editing"
                                        style={`padding-left: ${(file.level || 0) * 1.5 + 0.5}rem`}
                                >
                                    <span class="item-icon">
                                        {#if file.is_dir}
                                            <Folder size={16} />
                                        {:else}
                                            <File size={16} />
                                        {/if}
                                    </span>
                                    <input
                                            type="text"
                                            bind:value={newItemName}
                                            on:keydown={(e) => saveNewItem(e, file)}
                                            placeholder={file.is_dir ? "New Folder" : "New File"}
                                            autofocus
                                    />
                                </div>
                            {:else}
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
                            {/if}
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