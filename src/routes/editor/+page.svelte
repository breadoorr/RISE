<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/tauri";
    import Terminal from './Terminal.svelte';
    import {
        ChevronDown,
        ChevronRight,
        File,
        Folder,
        Terminal as TerminalIcon,
    } from "lucide-svelte";
    import FileMenu from "./FileMenu.svelte";
    import toggleFileMenu from './FileMenu.svelte';

    interface FileEntry {
        path: string;
        name: string;
        is_dir: boolean;
        expanded?: boolean;
        children?: FileEntry[];
        level?: number;
        parent_dir?: string;
    }

    let projectPath: string | null = null;
    let currentPath: string | null = null;
    let files: FileEntry[] = [];
    let allFiles: FileEntry[] = [];
    let selectedFile: string | null = null;
    let fileContent: string = '';
    let editorContent: string = '';
    let lastBufferContent: string = '';
    let isEdited: boolean = false;
    let lineCount: number = 1;
    let currentLine: number = 1;
    let currentColumn: number = 1;
    let editorElement: HTMLTextAreaElement;
    let editorWrapper: HTMLDivElement;
    let highlightContainer: HTMLDivElement;
    let highlightHtml: string = '';
    let highlightTimeout: number | null = null;
    let sidebarWidth: number = 300;
    let terminalHeight: number = 200;
    let autoSaveTimeout: number | null = null;
    let openFiles: FileEntry[] = [];
    let activeFileIndex: number = -1;
    let isSidebarOpen: boolean = true;
    let isTerminalOpen: boolean = false;
    let actions: Array<string> = [];
    let fileMenu;

    // user/system info
    let user: string = '';
    let host: string = '';
    let home: string = '';

    // Reference to toggleTerminal function from Terminal component
    let toggleTerminal: () => void;

    onMount(async () => {
        projectPath = localStorage.getItem('projectPath');
        if (projectPath) {
            currentPath = projectPath;
            const rootEntry: FileEntry = {
                path: projectPath,
                name: `${projectPath.split('/').pop()}`,
                is_dir: true,
                expanded: true,
                children: [],
                level: 0,
                parent_dir: ''
            };
            files = [rootEntry];
            updateAllFiles();
            await loadChildren(rootEntry);
        }

        const info = await invoke('get_system_info') as { user: string; host: string; home: string };
        user = info.user;
        host = info.host;
        home = info.home;

        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleInputEvent);
        window.addEventListener('click', handleInputEvent);
        window.addEventListener('focus', restoreEditorContent);

        syncLineNumbersScroll();

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', handleInputEvent);
            window.removeEventListener('click', handleInputEvent);
            window.removeEventListener('focus', restoreEditorContent);
            if (autoSaveTimeout !== null) {
                clearTimeout(autoSaveTimeout);
            }
        };
    });

    async function handleKeyDown(event: KeyboardEvent) {
        if (event.key === 'Tab' && event.target === editorElement) {
            event.preventDefault();
            const target = event.target as HTMLTextAreaElement;
            const start = target.selectionStart;
            const end = target.selectionEnd;
            const text = target.value;
            const before = text.slice(0, start);
            const after = text.slice(end);
            const indent = '    ';
            let result = before + indent + after;
            target.selectionStart = start + indent.length;
            lastBufferContent = result;
            editorContent = result;
            isEdited = result !== fileContent;
            if (editorElement) editorElement.value = result;
            updateLineNumbers(result);
            scheduleHighlight();
        }
        if (event.ctrlKey || event.metaKey) {
            if (event.key === 's') {
                event.preventDefault();
                if (selectedFile && isEdited) {
                    saveFile();
                }
            } else if (event.key === 'z') {
                event.preventDefault();
                if (!selectedFile) return;
                const result = await invoke('undo_last_change', { path: selectedFile }) as string;
                editorContent = result;
                lastBufferContent = result;
                isEdited = result !== fileContent;
                if (editorElement) editorElement.value = result;
                updateLineNumbers(result);
                scheduleHighlight();
            }
        } else if (event.key === 'Escape') {
            window.location.href = "/";
        }
    }

    async function loadFiles(path: string, level: number = 0) {
        try {
            const dirFiles = await invoke("list_files", { dirPath: path }) as FileEntry[];

            for (const file of dirFiles) {
                file.level = level;
                file.parent_dir = path.split('/').slice(-2, -1)[0] || '';
                if (file.is_dir) {
                    file.expanded = false;
                    file.children = [];
                }
            }

            if (level === 0) {
                files = dirFiles;
                updateAllFiles();
            }
            return dirFiles;
        } catch (error) {
            console.error("Error listing files:", error);
            return [];
        }
    }

    function updateAllFiles() {
        allFiles = [];
        function flattenFiles(fileList: FileEntry[]) {
            for (const file of fileList) {
                allFiles.push(file);
                if (file.is_dir && file.expanded && file.children) {
                    flattenFiles(file.children);
                }
            }
        }
        flattenFiles(files);
    }

    async function loadChildren(item: FileEntry) {
        if (!item.is_dir) return;
        try {
            item.children = await loadFiles(item.path, (item.level || 0) + 1);
            updateAllFiles();
        } catch (error) {
            console.error("Error loading children:", error);
        }
    }

    async function selectFile(item: FileEntry, event: MouseEvent) {
        event.preventDefault();
        if (event.button === 0) {
            event.preventDefault();
            if (item.is_dir) {
                item.expanded = !item.expanded;
                if (item.expanded && (!item.children || item.children.length === 0)) {
                    await loadChildren(item);
                }
                updateAllFiles();
                return;
            }

            if (autoSaveTimeout !== null) {
                clearTimeout(autoSaveTimeout);
            }

            const existingIndex = openFiles.findIndex(file => file.path === item.path);
            if (existingIndex !== -1) {
                activeFileIndex = existingIndex;
                await switchToFile(existingIndex);
            } else {
                openFiles = [...openFiles, item];
                activeFileIndex = openFiles.length - 1;
                await switchToFile(activeFileIndex);
            }
        } else if (event.button === 2) {
            event.preventDefault();
            actions = await invoke('get_actions', { isDir: item.is_dir });
            fileMenu.toggleFileMenu(event, actions);
            console.log(actions);
        }
    }

    function handleInputEvent(event: Event) {
        if (event.target === editorElement && editorElement.value !== undefined) {
            updateCurrentLine(event);
        }
    }

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
                updateLineNumbers(editorContent);
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
                updateLineNumbers(updated);
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

    async function switchToFile(index: number) {
        if (index < 0 || index >= openFiles.length) return;
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
        }

        activeFileIndex = index;
        const file = openFiles[index];
        selectedFile = file.path;

        try {
            fileContent = await invoke("open_buffer", { path: file.path }) as string;
            editorContent = fileContent;
            lastBufferContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
            if (editorElement) {
                editorElement.value = editorContent;
            }
            scheduleHighlight();
        } catch (error) {
            console.error("Error opening buffer:", error);
            fileContent = "Cannot display contents of the file";
            editorContent = fileContent;
            lastBufferContent = editorContent;
            isEdited = false;
            updateLineNumbers(fileContent);
            if (editorElement) {
                editorElement.value = editorContent;
            }
        }
    }

    function restoreEditorContent() {
        if (selectedFile && activeFileIndex >= 0 && editorElement) {
            editorElement.value = editorContent;
            updateLineNumbers(editorContent);
            currentLine = editorContent.slice(0, editorElement.selectionStart).split('\n').length;
        }
    }

    function closeFile(index: number, event: MouseEvent) {
        event.stopPropagation();
        if (index < 0 || index >= openFiles.length) return;

        if (activeFileIndex === index && isEdited) {
            if (!confirm("You have unsaved changes. Do you want to discard them?")) {
                return;
            }
        }

        openFiles = openFiles.filter((_, i) => i !== index);
        if (activeFileIndex === index) {
            if (openFiles.length > 0) {
                const newIndex = Math.min(index, openFiles.length - 1);
                switchToFile(newIndex);
            } else {
                activeFileIndex = -1;
                selectedFile = null;
                fileContent = '';
                editorContent = '';
                isEdited = false;
                updateLineNumbers('');
                if (editorElement) {
                    editorElement.value = '';
                }
            }
        } else if (activeFileIndex > index) {
            activeFileIndex--;
        }
    }

    function handleResize(event: MouseEvent) {
        const startX = event.clientX;
        const startWidth = sidebarWidth;
        const maxWidth = 600;

        function onMouseMove(moveEvent: MouseEvent) {
            const newWidth = startWidth + (moveEvent.clientX - startX);
            sidebarWidth = Math.max(100, Math.min(maxWidth, newWidth));
        }

        function onMouseUp() {
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', onMouseUp);
            syncLineNumbersScroll();
        }

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }

    function getDisplayName(file: FileEntry): string {
        const sameNameCount = openFiles.filter(f => f.name === file.name).length;
        if (sameNameCount > 1 && file.parent_dir) {
            return `${file.parent_dir}/${file.name}`;
        }
        return file.name;
    }
</script>

<main>
    <FileMenu bind:this={fileMenu} />
    <div class="sidebar--tools">
        <button class="sidebar--tools-item" class:active={isSidebarOpen} on:click={() => {
            sidebarWidth = isSidebarOpen ? 0 : 300;
            isSidebarOpen = sidebarWidth > 0;
        }}>
            <Folder size={25} />
        </button>
        <button class="sidebar--tools-item bottom" class:active={isTerminalOpen} on:click={toggleTerminal}>
            <TerminalIcon size={25} />
        </button>
    </div>
    <div class="sidebar" style="width: {sidebarWidth}px;">
        {#if projectPath}
            <div class="file-list" style="height: {isTerminalOpen ? `calc(100vh - ${terminalHeight}px)` : 'calc(100vh - 25px)'};">
                {#if allFiles.length > 0}
                    <ul>
                        {#each allFiles as file}
                            <li>
                                <button
                                        on:mousedown={(event) => selectFile(file, event)}
                                        class={'file-list-item ' + `${selectedFile === file.path ? 'selected' : ''} ${file.is_dir ? 'directory' : 'file'}`}
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

    <div class="editor-area" style="width: calc(100% - {sidebarWidth}px - 5px); height: {isTerminalOpen ? `calc(100% - ${terminalHeight}px - 25px)` : 'calc(100vh - 25px)'};">
        <div class="editor-header" class:editor-header--closed={openFiles.length === 0}>
            {#each openFiles as file, index}
                <div
                        class="file-tab"
                        class:active={index === activeFileIndex}
                >
                    <button class="file-tab--name-button" on:click={() => switchToFile(index)}>
                        <span class="file-tab-name">{getDisplayName(file)}</span>
                    </button>
                    <button
                            class="file-tab-close"
                            on:click={(e) => closeFile(index, e)}
                            title="Close file"
                    >×</button>
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

    <Terminal bind:isTerminalOpen bind:terminalHeight {projectPath} {user} {host} {home} bind:toggleTerminal />

    <div class="editor-footer">
        <div class="cursor-info">
            <p class="cursor-info--line-number">Line: {currentLine}</p>
            <p class="cursor-info--column-number">Col: {currentColumn}</p>
        </div>
    </div>
</main>

<style lang="scss">
  @use 'editor';
</style>