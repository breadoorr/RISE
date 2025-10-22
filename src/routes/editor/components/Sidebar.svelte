<script lang="ts">
    import { ChevronDown, ChevronRight, File, Folder, Terminal as TerminalIcon } from "lucide-svelte";
    import type { FileEntry } from "$lib/utils/types";
    import { createEventDispatcher } from "svelte";
    import { loadFiles as loadFilesUtil, updateAllFiles as flattenFilesUtil } from "$lib/utils/fileLoader";
    import FileMenu from "./FileMenu.svelte";
    import {invoke} from "@tauri-apps/api/tauri";
    import {path} from "@tauri-apps/api";
    import {refreshPathInStore} from "$lib/stores/fileStore";

    const dispatch = createEventDispatcher();
    let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;

    // Props from parent
    export let files: FileEntry[] = [];
    export let allFiles: FileEntry[] = [];
    export let selectedFilePath: string | null = null;
    export let projectPath: string | null = null;
    export let sidebarWidth: number = 300;
    export let isSidebarOpen: boolean = true;
    export let isTerminalOpen: boolean = false;
    export let terminalHeight: number = 200;

    let createNewItem: (isDir: boolean, parentPath: string, onNameConfirmed: (name: string) => Promise<void>) => void;
    let editItem: (isDir: boolean, path: string, onNameConfirmed: (name: string) => Promise<void>) => void;
    let moveItem: (path: string, newPath: string) => void;
    let editingItemPath: string | null = null;
    let newItemName: string = '';
    let errorMessage: string | null = null;
    let currentOnNameConfirmed: ((name: string) => Promise<void>) | null = null;
    let draggedItem: FileEntry | null = null;
    let dropTargetPath: string | null = null;

    // Regex for valid file/folder names: alphanumeric, spaces, hyphens, underscores, periods (for files)
    const validNameRegex = /^[a-zA-Z0-9 _-]([a-zA-Z0-9 _.-]*[a-zA-Z0-9 _-])?$/;
    const invalidChars = /[\/\\:*?"<>|]/;

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
        currentOnNameConfirmed = onNameConfirmed;
        const targetPath = parentPath || projectPath;
        const tempName = isDir ? "New Folder" : "New File";
        const tempPath = `${targetPath}/${tempName}`;

        const newItem: FileEntry = {
            parent_dir: parentPath || undefined,
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

        const position = allFiles.findIndex(f => f.path === parentPath);
        if (position !== -1) {
            allFiles.find(f => f.path === parentPath)!.expanded = true;
            allFiles = [...allFiles.slice(0, position + 1), newItem, ...allFiles.slice(position + 1)];
        } else {
            allFiles = [newItem, ...allFiles];
        }
        editingItemPath = tempPath;
        newItemName = "";
        errorMessage = null;
    };

    editItem = (isDir: boolean, path: string, onNameConfirmed: (name: string) => Promise<void>) => {
        const item = allFiles.find(f => f.path === path);
        if (item) {
            currentOnNameConfirmed = onNameConfirmed;
            item.isEditing = true;
            editingItemPath = path;
            newItemName = item.name;
            errorMessage = null;
            allFiles = [...allFiles]; // Force reactivity
            console.log("Editing item:", item);
        } else {
            console.error("Item not found for editing:", path);
        }
    };

    moveItem = () => {

    }

    function validateName(name: string, isDir: boolean, parentPath: string | undefined, currentPath: string | null): string | null {
        const trimmedName = name.trim();
        if (!trimmedName) {
            return "Name cannot be empty";
        }

        // Check for invalid characters
        if (invalidChars.test(trimmedName)) {
            return "Name cannot contain / \\ : * ? \" < > |";
        }

        // Check regex for valid name structure
        if (!validNameRegex.test(trimmedName)) {
            return "Name must start and end with alphanumeric, hyphen, underscore, or space";
        }

        // Check for duplicates in the parent directory, excluding the current item
        const parent = parentPath || projectPath;
        const siblings = allFiles.filter(f => (f.parent_dir === parent || (!f.parent_dir && parent === projectPath)) && f.path !== currentPath);
        if (siblings.some(f => f.name.toLowerCase() === trimmedName.toLowerCase() && f.is_dir === isDir)) {
            return isDir ? "A folder with this name already exists" : "A file with this name already exists";
        }

        return null;
    }

    function handleDragStart(event: DragEvent, file: FileEntry) {
        if (file.isEditing) return;
        draggedItem = file;
        event.dataTransfer!.setData('text/plain', file.path);
        event.dataTransfer!.effectAllowed = 'move';
    }

    async function handleDragOver(event: DragEvent, file: FileEntry) {
        event.preventDefault();
        event.stopPropagation();
        console.log("Over");
        if (!file.is_dir || !draggedItem || file.path === draggedItem.path || draggedItem.path.startsWith(`${file.path}/`)) return;
        if (!file.expanded) file.expanded = true;
        if (file.expanded && (!file.children || file.children.length === 0)) {
            try {
                file.children = await loadFilesUtil(file.path, (file.level || 0) + 1);
            } catch (e) {
                console.error('Failed to load children for', file.path, e);
            }
        }
        allFiles = flattenFilesUtil(files); // Refresh
        dispatch('filesChanged', { files });
        event.dataTransfer!.dropEffect = 'move';
        dropTargetPath = file.path; // For highlight
    }

    async function handleDragLeave(event: DragEvent, file: FileEntry) {
        event.preventDefault();
        const related = event.relatedTarget as HTMLElement | null;
        if (related && event.currentTarget && (event.currentTarget as HTMLElement).contains(related)) {
            return; // Still inside this button
        }

        console.log("Leave");
        dropTargetPath = null;
    }

    async function handleDrop(event: DragEvent, file: FileEntry) {
        event.preventDefault();
        // event.stopPropagation();
        console.log("Drop:", event, file, draggedItem);
        dropTargetPath = null;
        if (!draggedItem || !file.is_dir || file.path === draggedItem.path || draggedItem.path.startsWith(`${file.path}/`)) return;

        const newPath = await path.join(file.path, draggedItem.name);
        try {
            moveItem(draggedItem.path, newPath);
        } catch (e) {
            errorMessage = `Move failed: ${e}`;
        }
        draggedItem = null;
    }

    moveItem = async (sourcePath: string, newPath: string) => {
        console.log("Moving item:", sourcePath, "to", newPath);
        const item = allFiles.find(f => f.path === sourcePath);
        if (!item) throw new Error('Item not found');
        let action = item.is_dir ? "Move Folder" : "Move File"
        let res = await invoke('perform_action', { action, file: {path: sourcePath, name: item.name, is_dir: item.is_dir}, newName: newPath });
        console.log(res);
        await refreshPathInStore(sourcePath.split('/').slice(0, -1).join('/'));
        await refreshPathInStore(newPath.split('/').slice(0, -1).join('/'));
    };

    async function saveNewItem(event: KeyboardEvent, item: FileEntry) {
        if (event.key === "Enter") {
            const validationError = validateName(newItemName, item.is_dir, item.parent_dir, item.temp ? null : item.path);
            if (validationError) {
                errorMessage = validationError;
                return;
            }

            try {
                // Update the item in the UI
                item.name = newItemName.trim();
                item.path = `${item.path.split('/').slice(0, -1).join('/')}/${item.name}`;
                item.isEditing = false;
                item.temp = false;

                // Call the stored callback with the final name
                if (currentOnNameConfirmed) {
                    await currentOnNameConfirmed(item.name);
                }

                // Reset editing state
                editingItemPath = null;
                newItemName = "";
                errorMessage = null;
                currentOnNameConfirmed = null;

                console.log(item.temp ? "Created item:" : "Renamed item:", item);

                // Optionally open the file
                if (!item.is_dir) {
                    dispatch("openFile", { file: item });
                }

                // Recompute flattened list and notify parent to sync store
                allFiles = flattenFilesUtil(files);
                dispatch('filesChanged', { files });
            } catch (e) {
                console.error("Failed to", item.temp ? "create" : "rename", "item:", e);
                if (item.temp) {
                    allFiles = allFiles.filter(f => f.path !== item.path);
                }
                errorMessage = `Failed to ${item.temp ? "create" : "rename"} item. Please try again.`;
                currentOnNameConfirmed = null;
            }
        } else if (event.key === "Escape") {
            if (item.temp) {
                allFiles = allFiles.filter(f => f.path !== item.path);
            } else {
                item.isEditing = false;
            }
            editingItemPath = null;
            newItemName = "";
            errorMessage = null;
            currentOnNameConfirmed = null;
        }
    }

    // Reactively update error message as the user types
    $: {
        if (newItemName && editingItemPath) {
            const item = allFiles.find(f => f.path === editingItemPath);
            if (item) {
                errorMessage = validateName(newItemName, item.is_dir, item.parent_dir, item.temp ? null : item.path);
            }
        } else {
            errorMessage = null;
        }
    }

    async function onSelectFile(file: FileEntry, event: MouseEvent) {
        if (event.button === 0) {
            toggleFileMenu(event, false);
            if (file.is_dir) {
                file.expanded = !file.expanded;
                if (file.expanded && (!file.children || file.children.length === 0)) {
                    try {
                        file.children = await loadFilesUtil(file.path, (file.level || 0) + 1);
                    } catch (e) {
                        console.error('Failed to load children for', file.path, e);
                    }
                }
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
                style="height: {isTerminalOpen ? `calc(100vh - ${terminalHeight+25}px)` : 'calc(100vh - 25px)'};"
        >
            <FileMenu bind:toggleFileMenu bind:createNewItem bind:editItem bind:moveItem />

            {#if allFiles.length > 0}
                <ul>
                    {#each allFiles as file}
                        <li

                                class:drop-target={file.is_dir && dropTargetPath === file.path}
                        >
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
                                {#if errorMessage}
                                    <p class="error-message">{errorMessage}</p>
                                {/if}
                            {:else}
                                <button
                                        draggable="true"
                                        on:dragenter={(e) => e.preventDefault()}
                                        on:dragstart={(e) => handleDragStart(e, file)}
                                        on:dragover={(e) => handleDragOver(e, file)}
                                        on:dragleave={(e) => handleDragLeave(e, file)}
                                        on:dragend={() => {if (draggedItem && dropTargetPath) {
                                            handleDrop(new DragEvent("drop"), { name: "", path: dropTargetPath, is_dir: true });
                                        }}}
                                        on:drop={(e) => handleDrop(e, file)}
                                        on:mousedown={(event) => onSelectFile(file, event)}
                                        class={`file-list-item ${selectedFilePath === file.path ? 'selected' : ''} ${file.is_dir ? 'directory' : 'file'}`}
                                        style={`padding-left: ${(file.level || 0) * 1.5 + 0.5}rem;`}
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
  .drop-target .file-list-item { background: var(--secondary-300); }
</style>