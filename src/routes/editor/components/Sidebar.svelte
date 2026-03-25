<script lang="ts">
    import { ChevronDown, ChevronRight, File, Folder, Terminal as TerminalIcon } from "lucide-svelte";
    import type { FileEntry } from "$lib/utils/types";
    import { createEventDispatcher } from "svelte";
    import { loadFiles as loadFilesUtil, updateAllFiles as flattenFilesUtil } from "$lib/utils/fileLoader";
    import FileMenu from "./FileMenu.svelte";
    import {invoke} from "@tauri-apps/api/core";
    import { join, dirname } from "@tauri-apps/api/path";
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
    let isDragging: boolean = false;
    let pressedItem: FileEntry | null = null;
    let pressStartX = 0;
    let pressStartY = 0;
    let suppressClick = false;
    let dragX = 0;
    let dragY = 0;

    // Action to focus an element on mount without using the autofocus attribute (avoids a11y warning)
    function focusOnMount(node: HTMLElement) {
        const t = setTimeout(() => node.focus(), 0);
        return { destroy() { clearTimeout(t); } };
    }

    // Regex for valid file/folder names: alphanumeric, spaces, hyphens, underscores, periods (for files)
    const validNameRegex = /^[a-zA-Z0-9 _-]([a-zA-Z0-9 _.-]*[a-zA-Z0-9 _-])?$/;
    const invalidChars = /[\/\\:*?"<>|]/;

    // Safe wrapper around dirname that returns null if the path has no parent or is invalid
    async function safeDirname(p: string): Promise<string | null> {
        try {
            return await dirname(p);
        } catch (e) {
            // e.g., virtual/temp paths or root without parent
            return null;
        }
    }

    // Cross-platform check: is `child` located within `parent` (or equal)
    async function isDescendant(parent: string, child: string): Promise<boolean> {
        if (!parent || !child) return false;
        if (parent === child) return true;
        let cur = child;
        while (true) {
            const next = await safeDirname(cur);
            if (!next || next === cur) break; // reached root or invalid
            if (next === parent) return true;
            cur = next;
        }
        return false;
    }

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
        // Use a unique temporary path token to avoid OS-specific separators and collisions
        const tempPath = `${targetPath}::${tempName}::temp-${Date.now()}`;

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

    // Debounce timers for auto-expanding folders during hover (per-path)
    const expandTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();

    function startPress(file: FileEntry, event: MouseEvent) {
        if (file.isEditing || (file as any).temp) return; // block dragging for editing/temp items
        if (event.button !== 0) return; // only left button
        pressedItem = file;
        pressStartX = event.clientX;
        pressStartY = event.clientY;
        dragX = event.clientX + 8;
        dragY = event.clientY + 8;
        isDragging = false;
        window.addEventListener('mousemove', handleGlobalMouseMove);
        window.addEventListener('mouseup', handleGlobalMouseUp, { once: true });
    }

    function handleGlobalMouseMove(event: MouseEvent) {
        if (!pressedItem) return;
        const dx = Math.abs(event.clientX - pressStartX);
        const dy = Math.abs(event.clientY - pressStartY);
        const threshold = 5;
        if (!isDragging && (dx > threshold || dy > threshold)) {
            isDragging = true;
            draggedItem = pressedItem;
        }
        if (!isDragging) return;
        // update ghost position regardless, so it appears as soon as dragging starts
        dragX = event.clientX + 8;
        dragY = event.clientY + 8;
        if (!draggedItem) return;
        const targetPath = findHoverTargetPath(event.clientX, event.clientY);
        updateDropTarget(targetPath);
    }

    async function handleGlobalMouseUp(event: MouseEvent) {
        window.removeEventListener('mousemove', handleGlobalMouseMove);
        const wasDragging = isDragging;
        isDragging = false;
        const source = draggedItem;
        const targetPath = dropTargetPath;
        pressedItem = null;
        draggedItem = null;
        const finalTargetPath = findHoverTargetPath(event.clientX, event.clientY) || targetPath;
        // Clear hover highlight
        updateDropTarget(null);
        if (wasDragging && source && finalTargetPath) {
            const target = allFiles.find(f => f.path === finalTargetPath);
            if (target && target.is_dir && source.path !== target.path && !(await isDescendant(source.path, target.path))) {
                try {
                    const newPath = await join(target.path, source.name);
                    await moveItem(source.path, newPath);
                } catch (e) {
                    errorMessage = `Move failed: ${e}`;
                }
            }
            suppressClick = true; // prevent click after drag
        }
    }

    function findHoverTargetPath(x: number, y: number): string | null {
        const el = document.elementFromPoint(x, y) as HTMLElement | null;
        if (!el) return null;
        let cur: HTMLElement | null = el;
        while (cur) {
            const path = cur.getAttribute?.('data-path');
            const isDirAttr = cur.getAttribute?.('data-isdir');
            if (path && isDirAttr === 'true') {
                return path;
            }
            cur = cur.parentElement as HTMLElement | null;
        }
        return null;
    }

    async function updateDropTarget(targetPath: string | null) {
        if (dropTargetPath === targetPath) return;
        dropTargetPath = targetPath;
        // Schedule auto-expand if hovering a folder
        if (targetPath) {
            const file = allFiles.find(f => f.path === targetPath);
            if (file && file.is_dir) {
                const existing = expandTimers.get(targetPath);
                if (existing) clearTimeout(existing);
                const timer = setTimeout(async () => {
                    try {
                        if (!file.expanded) file.expanded = true;
                        if (file.expanded && (!file.children || file.children.length === 0)) {
                            file.children = await loadFilesUtil(file.path, (file.level || 0) + 1);
                            allFiles = flattenFilesUtil(files);
                            dispatch('filesChanged', { files });
                        }
                    } catch (e) {
                        console.error('Failed to load children for', file.path, e);
                    }
                }, 500);
                expandTimers.set(targetPath, timer);
            }
        }
    }

    moveItem = async (sourcePath: string, newPath: string) => {
        console.log("Moving item:", sourcePath, "to", newPath);
        const item = allFiles.find(f => f.path === sourcePath);
        if (!item) throw new Error('Item not found');
        let action = item.is_dir ? "Move Folder" : "Move File";
        let res = await invoke('perform_action', { action, file: { path: sourcePath, name: item.name, is_dir: item.is_dir }, newName: newPath });
        console.log(res);
        const oldParent = await safeDirname(sourcePath);
        const newParent = await safeDirname(newPath);
        if (oldParent) await refreshPathInStore(oldParent);
        if (newParent && newParent !== oldParent) await refreshPathInStore(newParent);
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
                const parentDir = item.parent_dir ?? await dirname(item.path);
                item.path = await join(parentDir, item.name);
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
        // If a drag just happened, suppress the click action to avoid toggling/opening
        if (suppressClick) {
            suppressClick = false;
            return;
        }
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
                style="height: {isTerminalOpen ? `calc(100vh - ${terminalHeight+60}px)` : 'calc(100vh - 60px)'};"
        >
            <FileMenu bind:toggleFileMenu bind:createNewItem bind:editItem />

            {#if allFiles.length > 0}
                <ul>
                    {#each allFiles as file}
                        <li
                                data-path={file.path}
                                data-isdir={file.is_dir}
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
                                            use:focusOnMount
                                    />
                                </div>
                                {#if errorMessage}
                                    <p class="error-message">{errorMessage}</p>
                                {/if}
                            {:else}
                                <button
                                        on:mousedown={(event) => startPress(file, event)}
                                        on:click={(event) => onSelectFile(file, event)}
                                        on:contextmenu={(event) => { event.preventDefault(); toggleFileMenu(event, true, file.is_dir, file.path, projectPath); }}
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

{#if isDragging && draggedItem}
    <div class="drag-ghost" style={`top: ${dragY}px; left: ${dragX}px;`}>
        <span class="item-icon">
            {#if draggedItem.is_dir}
                <Folder size={14} />
            {:else}
                <File size={14} />
            {/if}
        </span>
        <span class="name">{draggedItem.name}</span>
    </div>
{/if}

<button aria-label="resizer" class="resizer" on:mousedown={handleResize}></button>

<style lang="scss">
  @use "../style/sidebar.scss";

</style>