<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api";
    import {ChevronDown, ChevronRight} from "lucide-svelte";

    // Define the FileEntry interface to match the Rust struct
    interface FileEntry {
        path: string;
        name: string;
        is_dir: boolean;
        expanded?: boolean;
        children?: FileEntry[];
        level?: number;
    }

    let projectPath: string | null = null;
    let currentPath: string | null = null;
    let files: FileEntry[] = [];
    let allFiles: FileEntry[] = []; // Flat list of all files for display
    let selectedFile: string | null = null;
    let fileContent: string = '';
    let editorContent: string = '';
    let isEdited: boolean = false;
    let lineCount: number = 1;
    let lineNumbers: string = '';
    let editorElement: HTMLTextAreaElement;
    let editorWrapper: HTMLDivElement;
    let sidebarWidth: number = 30; // Initial width in percentage
    let autoSaveTimeout: number | null = null; // For auto-save debounce
    let openFiles: FileEntry[] = []; // Buffer of open files
    let activeFileIndex: number = -1; // Index of the currently active file in the buffer
    let currentLine = 1

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
                level: 0
            };

            files = [rootEntry];
            updateAllFiles();

            await loadChildren(rootEntry);
        }

        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', updateCurrentLine);
        window.addEventListener('click', updateCurrentLine);
        window.addEventListener('resize', syncLineNumbersScroll);

        // Initial sync after the component is mounted
        setTimeout(syncLineNumbersScroll, 0);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', updateCurrentLine);
            window.removeEventListener('click', updateCurrentLine);
            window.removeEventListener('resize', syncLineNumbersScroll);

            if (autoSaveTimeout !== null) {
                clearTimeout(autoSaveTimeout);
                autoSaveTimeout = null;
            }
        };
    });

    function handleKeyDown(event: KeyboardEvent) {
        // Ctrl+S or Cmd+S to save
        if ((event.ctrlKey || event.metaKey) && event.key === 's') {
            event.preventDefault();
            if (selectedFile && isEdited) {
                saveFile();
            }
        } else if (event.key === 'Escape') {
            goToWelcomeScreen();
        } else {
            handleEditorChange(event);
        }
    }

    function goToWelcomeScreen() {
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
            autoSaveTimeout = null;
        }

        window.location.href = "/";
    }

    async function loadFiles(path?: string, level: number = 0) {
        const dirPath = path || currentPath;
        if (!dirPath) return [];

        try {
            const dirFiles = await invoke("list_files", { dirPath }) as FileEntry[];

            // Sort files and directories (directories first)
            dirFiles.sort((a, b) => {
                if (a.is_dir && !b.is_dir) return -1;
                if (!a.is_dir && b.is_dir) return 1;
                return a.name.localeCompare(b.name);
            });

            // Add level and expanded properties
            for (const file of dirFiles) {
                file.level = level;
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

    // Function to update the flattened file list for display
    function updateAllFiles() {
        allFiles = [];

        function flattenFiles(fileList: FileEntry[]) {
            for (const file of fileList) {
                allFiles.push(file);
                if (file.is_dir && file.expanded && file.children && file.children.length > 0) {
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

    async function selectFile(item: FileEntry) {
        // If it's a directory, toggle expansion
        if (item.is_dir) {
            item.expanded = !item.expanded;

            // Load children if expanding and children haven't been loaded yet
            if (item.expanded && (!item.children || item.children.length === 0)) {
                await loadChildren(item);
            }

            updateAllFiles();
            return;
        }

        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
            autoSaveTimeout = null;
        }

        if (isEdited) {
            if (!confirm("You have unsaved changes. Do you want to discard them?")) {
                return;
            }
        }

        // Check if the file is already open
        const existingIndex = openFiles.findIndex(file => file.path === item.path);

        if (existingIndex !== -1) {
            // If the file is already open, just switch to it
            activeFileIndex = existingIndex;
        } else {
            // Add the file to the open files buffer
            openFiles = [...openFiles, item];
            activeFileIndex = openFiles.length - 1;
        }

        selectedFile = item.path;
        try {
            fileContent = await invoke("read_file", { path: item.path });
            editorContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
        } catch (error) {
            console.error("Error reading file:", error);
            // Set a message for non-displayable files
            fileContent = "Cannot display contents of the file";
            editorContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
        }
    }

    function updateCurrentLine(event: Event) {
        const target = event.target as HTMLTextAreaElement;
        editorContent = target.value;
        currentLine = editorContent.slice(0, target.selectionStart).split('\n').length;
    }

    function handleEditorChange(event: Event) {
        const target = event.target as HTMLTextAreaElement;
        editorContent = target.value;
        isEdited = editorContent !== fileContent;
        updateLineNumbers(editorContent);
        updateCurrentLine(event);

        if (isEdited && selectedFile) {
            if (autoSaveTimeout !== null) {
                clearTimeout(autoSaveTimeout);
            }

            autoSaveTimeout = setTimeout(() => {
                autoSave();
                autoSaveTimeout = null;
            }, 500);
        }
    }

    async function autoSave() {
        if (isEdited && selectedFile && fileContent !== "Cannot display contents of the file") {
            try {
                await invoke("write_file", { path: selectedFile, content: editorContent });
                fileContent = editorContent;
                isEdited = false;
                console.log("Auto-saved file successfully");
            } catch (error) {
                console.error("Error auto-saving file:", error);
            }
        }
    }

    function updateLineNumbers(text: string) {
        const lines = text.split('\n').length;
        lineCount = lines;
        lineNumbers = Array.from({ length: lines }, (_, i) => i + 1).join('\n');

        // Sync scroll position between line numbers and editor
        syncLineNumbersScroll();
    }

    function syncLineNumbersScroll() {
        if (editorWrapper && editorElement) {
            const lineNumbersContent = editorWrapper.querySelector('.line-numbers-content') as HTMLDivElement;
            if (lineNumbersContent) {
                // Use transform to move the line numbers content container
                lineNumbersContent.style.transform = `translateY(-${editorElement.scrollTop}px)`;

                // Ensure the line numbers container has the same height as the editor content
                const editorContentHeight = editorElement.scrollHeight;
                lineNumbersContent.style.height = `${editorContentHeight}px`;
            }
        }
    }

    function handleEditorScroll() {
        syncLineNumbersScroll();
    }

    async function saveFile() {
        if (!selectedFile) return;

        try {
            await invoke("write_file", { path: selectedFile, content: editorContent });
            fileContent = editorContent;
            isEdited = false;
            alert("File saved successfully!");
        } catch (error) {
            console.error("Error saving file:", error);
            alert("Error saving file: " + error);
        }
    }

    function getFileExtension(filename: string): string {
        return filename.split('.').pop()?.toLowerCase() || '';
    }

    function getFileType(filename: string): string {
        const ext = getFileExtension(filename);
        switch (ext) {
            case 'js':
                return 'JavaScript';
            case 'ts':
                return 'TypeScript';
            case 'html':
                return 'HTML';
            case 'css':
                return 'CSS';
            case 'json':
                return 'JSON';
            case 'md':
                return 'Markdown';
            case 'rs':
                return 'Rust';
            case 'svelte':
                return 'Svelte';
            default:
                return ext.toUpperCase();
        }
    }

    // Function to switch to a specific file in the buffer
    async function switchToFile(index: number) {
        if (index < 0 || index >= openFiles.length) return;

        // Check for unsaved changes first
        if (isEdited) {
            if (!confirm("You have unsaved changes. Do you want to discard them?")) {
                return;
            }
        }

        // Clear any pending auto-save timeout
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
            autoSaveTimeout = null;
        }

        activeFileIndex = index;
        const file = openFiles[index];
        selectedFile = file.path;

        try {
            fileContent = await invoke("read_file", { path: file.path });
            editorContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
        } catch (error) {
            console.error("Error reading file:", error);
            fileContent = "Cannot display contents of the file";
            editorContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
        }
    }

    // Function to close a file from the buffer
    function closeFile(index: number, event: MouseEvent) {
        // Stop the click event from propagating to the tab
        event.stopPropagation();

        if (index < 0 || index >= openFiles.length) return;

        // If the file has unsaved changes, confirm before closing
        if (activeFileIndex === index && isEdited) {
            if (!confirm("You have unsaved changes. Do you want to discard them?")) {
                return;
            }
        }

        // Remove the file from the buffer
        openFiles = openFiles.filter((_, i) => i !== index);

        // If we closed the active file
        if (activeFileIndex === index) {
            // If there are still files open, switch to the last one
            if (openFiles.length > 0) {
                const newIndex = Math.min(index, openFiles.length - 1);
                switchToFile(newIndex);
            } else {
                // No files left open
                activeFileIndex = -1;
                selectedFile = null;
                fileContent = '';
                editorContent = '';
                isEdited = false;
                updateLineNumbers('');
            }
        } else if (activeFileIndex > index) {
            // If we closed a file before the active one, adjust the active index
            activeFileIndex--;
        }
    }

    // Handle sidebar resizing
    function handleResize(event: MouseEvent) {
        // Start resizing
        const startX = event.clientX;
        const startWidth = sidebarWidth;

        function onMouseMove(moveEvent: MouseEvent) {
            // Calculate new width as a percentage of the window width
            const newWidth = startWidth + ((moveEvent.clientX - startX) / window.innerWidth) * 100;

            // Limit the sidebar width between 10% and 90%
            sidebarWidth = Math.max(10, Math.min(90, newWidth));
        }

        function onMouseUp() {
            // Stop resizing
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', onMouseUp);

            // Sync line numbers after resizing
            syncLineNumbersScroll();
        }

        // Add event listeners for mouse move and mouse up
        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }
</script>

<main>
    <div class="sidebar" style="width: {sidebarWidth}%">
        {#if projectPath}
            <div class="file-list">
                {#if allFiles.length > 0}
                    <ul>
                        {#each allFiles as file}
                                <li><button on:click={() => selectFile(file)} class={'file-list-item ' + `${selectedFile === file.path ? 'selected' : ''} ${file.is_dir ? 'directory' : 'file'}`}
                                     style={`padding-left: ${(file.level || 0) * 1.5 + 0.5}rem`}>
                                <span class="item-icon">
                                    {#if file.is_dir}
                                        {#if file.expanded}
                                            <ChevronDown size={16} />
                                        {:else}
                                            <ChevronRight size={16} />
                                        {/if}
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

    <div class="resizer" on:mousedown={handleResize}></div>

    <div class="editor-area" style="width: calc(100% - {sidebarWidth}% - 5px)">
            <div class="editor-header" class:editor-header--closed={openFiles.length === 0}>
                {#each openFiles as file, index}
                    <div
                        class="file-tab" 
                        class:active={index === activeFileIndex}
                        on:click={() => switchToFile(index)}
                    >
                        <span class="file-tab-name">{file.name}</span>
                        <button 
                            class="file-tab-close" 
                            on:click={(e) => closeFile(index, e)}
                            title="Close file"
                        >×</button>
                    </div>
                {/each}
            </div>

            {#if selectedFile}
                <div class="editor-wrapper" bind:this={editorWrapper}>
                    <div class="line-numbers">
                        <div class="line-numbers-content">
                            {#each Array(lineCount) as _, i}
                                <div class="line-number" class:active={i + 1 === currentLine}>{i + 1}</div>
                            {/each}
                        </div>
                    </div>
                    <textarea 
                        class={`code-editor ${fileContent === "Cannot display contents of the file" ? 'no-file-selected' : ''}`}
                        bind:value={editorContent}
                        bind:this={editorElement}
                        on:input={handleEditorChange}
                        on:scroll={handleEditorScroll}
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

    <div class="editor-footer">
        <div class="cursor-info">
            <p class="cursor-info--line-number">Line: {currentLine}</p>
            <p class="cursor-info--column-number">Col: </p>
        </div>
    </div>
</main>

<style lang="scss">
    :global(html), :global(body) {
        margin: 0;
        padding: 0;
        height: 100%;
        width: 100%;
        overflow: hidden;
        --webkit-user-select: none;
    }

    :global(body) {
        color: var(--accent-dark);
        background-color: var(--white);
        font-family: sans-serif;
      user-select: none;
    }

    main {
        margin: 0;
        padding: 0;
        height: 100vh;
        width: 100vw;
        display: flex;
        overflow: hidden;
        user-select: none;
    }

    .sidebar {
        background-color: var(--grey);
        height: 100vh;
        user-select: none;
    }

    .resizer {
        width: 2px;
        height: 100vh;
        background-color: rgb(193, 193, 193);
        cursor: col-resize;
        user-select: none;
    }

    .file-list {
      display: block;
      height: 100vh;
      user-select: none;
    }

    .editor-area {
        height: 100vh;
        overflow: hidden;
        user-select: none;
    }

    .editor-header {
      position: relative;
      z-index: 1;
      display: flex;
      background-color: white;
      border-bottom: 1px solid var(--stroke-grey);
      overflow-x: scroll;
      overflow-y: hidden;
      white-space: nowrap;
      user-select: none;

      &--closed {
        display: none;
      }

      &::-webkit-scrollbar {
        display: none;
      }
    }

    .file-tab {
      display: flex;
      align-items: center;
      padding: 0 10px;
      height: 30px;
      background-color: white;
      color: gray;
      cursor: pointer;
      user-select: none;

      &:hover {
        color: black;
      }

      &.active {
        color: black;
        border-bottom: solid 3px var(--accent-green);
      }

      &-name {
        margin-left: 8px;
        margin-right: 6px;
        font-size: 0.9rem;
        user-select: none;
      }

      &-close {
        background: none;
        border: none;
        color: white;
        font-size: 1.2rem;
        line-height: 1;
        padding: 0;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        user-select: none;

        &:hover {
          background-color: rgba(255, 255, 255, 0.1);
          color: white;
        }
      }

      &.active > &-close {
        color: var(--grey);
      }

      &:hover > &-close {
        color: var(--grey);
      }
    }

    .editor-footer {
      width: 100%;
      height: 25px;
      position: absolute;
      left:0;
      bottom:0;
      background-color: var(--white);
      border-top: solid 2px var(--stroke-grey);
      user-select: none;
    }

    .cursor-info {
      position: absolute;
      bottom: -7px;
      right: 0;
      display: block;
      padding: 0;
      font-size: 0.8rem;
      margin-right: 40px;
      color: gray;

      &--line-number {
        display: inline-block;
        margin-right: 5px;
      }
      &--column-number {
        display: inline-block;
        margin-right: 5px;
      }
    }

    .file-info {
        flex: 1;
        font-size: 0.9rem;
        color: #555;
        display: flex;
        align-items: center;
        user-select: none;
    }

    .editor-wrapper {
        flex: 1;
        display: flex;
        overflow: hidden;
        background-color: #ffffff;
        height: 100%;
    }

    .line-numbers {
      padding: 0;
      background-color: var(--white);
      font-family: monospace;
      font-size: 14px;
      line-height: 1.5;
      color: #999;
      user-select: none;
      overflow: hidden;
      position: relative;
      height: 100%;

      &-content {
        user-select: none;
        padding: 1rem 0.5rem 1rem 0.5rem;
        margin-bottom: 50px;
      }
    }

    .line-number.active {
      color: black;
    }

    .code-editor {
      z-index: 90;
        flex: 1;
        padding: 1rem;
        font-family: monospace;
        font-size: 14px;
        line-height: 1.5;
        border: none;
        resize: none;
        outline: none;
        background-color: #ffffff;
        color: #333;
        tab-size: 4;
        white-space: pre;
        overflow-x: auto;
        overflow-y: auto;
        margin-bottom: 70px;
        user-select: text;
    }

    .code-editor.non-displayable, .code-editor.no-file-selected {
        font-family: sans-serif;
        font-size: 16px;
        color: #e74c3c;
        text-align: center;
        display: flex;
        align-items: center;
        justify-content: center;
        user-select: none;
    }

    .no-file-selected {
        flex: 1;
        display: flex;
        justify-content: center;
        align-items: center;
        background-color: #ffffff;
        color: #999;
        height: 100%;
        user-select: none;
    }

    h2 {
        color: #333;
        margin-top: 0;
        margin-bottom: 1rem;
        user-select: none;
    }

    .project-path {
        font-size: 0.8rem;
        color: #666;
        margin-bottom: 1rem;
        word-break: break-all;
        user-select: none;
    }

    .file-list {
        overflow-y: scroll;

      &-item {
        cursor: pointer;
        border-radius: 3px;
        border: none;
        margin-bottom: 0.25rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        display: flex;
        align-items: center;
        transition: background-color 0.2s ease;
        width: 100%;
        min-width: 100px;
        background-color: var(--grey);
        font-size: 0.9rem;
        user-select: none;

        &.selected {
          background-color: var(--accent-dark);
          color: white;
          margin-right: 10px;
        }
      }
    }

    .file-list ul {
        list-style: none;
        padding-left: 0;
        margin-bottom: 50px;
        user-select: none;
    }

    .item-icon {
        margin-right: 0.5rem;
        font-size: 1.1rem;
        min-width: 1.5rem;
        text-align: center;
        user-select: none;
    }

    @media (prefers-color-scheme: dark) {
        :global(body) {
            background-color: var(--background-dark);
            color: #c1c1c1;
        }

        .sidebar {
            background-color: var(--sidebar-dark);
            border-right-color: var(--stroke-dark);
        }

      .resizer {
        background-color: var(--background-dark);
      }

        .editor-header {
            background-color: var(--bar-dark);
            border-bottom: 1px solid var(--stroke-dark);
        }

        .file-tab {
          background-color: var(--bar-dark);
          color: #aaa;
          border-right-color: #1a1a1a;

          &:hover {
            color: #fff;
          }

          &.active {
            color: #fff;
            border-bottom: solid 3px var(--accent-green);
          }

          &-close {
            color: var(--bar-dark);

            &:hover {
              background-color: rgba(255, 255, 255, 0.15);
            }
          }

          &.active > &-close {
            color: var(--grey);
          }

          &:hover > &-close {
            color: var(--grey);
          }
        }

      .editor-footer {
        background-color: var(--bar-dark);
        border-top: solid 2px var(--stroke-dark);
      }

      .cursor-info {
        color: #aaa;
      }

        .line-numbers {
            background-color: var(--background-dark);
            color: #777;
        }

        .line-number.active {
            color: #fff;
        }

        .line-numbers-content {
            padding: 1rem 0.5rem 1rem 0.5rem;
        }

        .file-type {
            background-color: #444;
            color: #ddd;
        }

        .editor-wrapper {
            background-color: var(--background-dark);

        }

        .code-editor {
            background-color: var(--background-dark);
            color: #e1e1e1;
        }

        .code-editor.non-displayable {
            background-color: #2a2a2a;
            color: #e74c3c;
        }

        .no-file-selected {
            background-color: #1e1e1e;
            color: #777;
        }

        h2, .file-info {
            color: #e0e0e0;
        }

        .project-path {
            color: #aaa;
        }

        .file-list {
          &-item {
            color: #aaa;
            background-color: var(--sidebar-dark);

            &.selected {
              background-color: #2c3e50;
              color: #3498db;
            }
          }
        }

        /* Navigation controls removed as we're using a hierarchical view */
    }
</style>
