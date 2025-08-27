<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api";
    import { Terminal } from 'xterm';
    import { FitAddon } from 'xterm-addon-fit';
    import { WebLinksAddon } from 'xterm-addon-web-links';
    import 'xterm/css/xterm.css';
    import {
        ChevronDown,
        ChevronRight,
        File,
        Folder,
        Terminal as TerminalIcon,
    } from "lucide-svelte";

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
    let isEdited: boolean = false;
    let lineCount: number = 1;
    let currentLine: number = 1;
    let currentColumn: number = 1;
    let editorElement: HTMLTextAreaElement;
    let editorWrapper: HTMLDivElement;
    let sidebarWidth: number = 300;
    let terminalHeight: number = 200;
    let autoSaveTimeout: number | null = null;
    let openFiles: FileEntry[] = [];
    let activeFileIndex: number = -1;
    let isSidebarOpen: boolean = true;
    let isTerminalOpen: boolean = false;
    let terminalElement: HTMLDivElement;
    let terminal: Terminal | null = null;
    let fitAddon: FitAddon | null = null;
    let commandBuffer: string = '';
    let history: string[] = [];
    let historyIndex: number = -1;
    let user: string = '';
    let host: string = '';
    let home: string = '';
    let currentCwd: string = '';

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
        currentCwd = projectPath || home;

        if (terminalElement) {
            terminal = new Terminal({
                theme: { background: '#1e1e1e', foreground: '#ffffff' },
                fontFamily: 'monospace',
                fontSize: 14,
                allowTransparency: true,
                convertEol: true,
                scrollback: 1000,
            });

            fitAddon = new FitAddon();
            terminal.loadAddon(fitAddon);
            terminal.loadAddon(new WebLinksAddon());

            terminal.open(terminalElement);
            terminal.write('Welcome to RISE IDE Terminal!\r\n' + getPrompt());
            terminal.scrollToBottom();

            terminal.onKey((event) => {
                const ev = event.domEvent;
                const key = ev.key;

                if (key === 'Enter') {
                    terminal.write('\r\n');
                    const command = commandBuffer.trim();
                    if (command) {
                        history.push(command);
                        historyIndex = history.length;
                        if (command.startsWith('cd ')) {
                            handleCdCommand(command);
                        } else {
                            invoke('execute_command', { cmd: command, cwd: currentCwd })
                                .then((output: string) => {
                                    terminal?.write(`${output}\r\n${getPrompt()}`);
                                    terminal?.scrollToBottom();
                                })
                                .catch((error) => {
                                    terminal?.write(`Error: ${error}\r\n${getPrompt()}`);
                                    terminal?.scrollToBottom();
                                });
                        }
                    } else {
                        terminal.write(getPrompt());
                        terminal.scrollToBottom();
                    }
                    commandBuffer = '';
                } else if (key === 'Backspace') {
                    if (commandBuffer.length > 0) {
                        commandBuffer = commandBuffer.slice(0, -1);
                        terminal.write('\b \b');
                        terminal.scrollToBottom();
                    }
                } else if (key === 'ArrowUp') {
                    if (historyIndex > 0) {
                        historyIndex--;
                        updateCommandLine(history[historyIndex]);
                        terminal.scrollToBottom();
                    }
                } else if (key === 'ArrowDown') {
                    if (historyIndex < history.length - 1) {
                        historyIndex++;
                        updateCommandLine(history[historyIndex]);
                    } else if (historyIndex === history.length - 1) {
                        historyIndex = history.length;
                        updateCommandLine('');
                    }
                    terminal.scrollToBottom();
                } else if (ev.ctrlKey && key.toLowerCase() === 'l') {
                    terminal.reset();
                    terminal.write(getPrompt());
                    commandBuffer = '';
                    terminal.scrollToBottom();
                } else if (!ev.ctrlKey && !ev.altKey && !ev.metaKey && key.length === 1 && key >= ' ' && key <= '~') {
                    commandBuffer += key;
                    terminal.write(key);
                    terminal.scrollToBottom();
                }
            });
        }

        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleInputEvent);
        window.addEventListener('click', handleInputEvent);
        window.addEventListener('resize', handleWindowResize);
        window.addEventListener('focus', restoreEditorContent);

        syncLineNumbersScroll();

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', handleInputEvent);
            window.removeEventListener('click', handleInputEvent);
            window.removeEventListener('resize', handleWindowResize);
            window.removeEventListener('focus', restoreEditorContent);
            if (autoSaveTimeout !== null) {
                clearTimeout(autoSaveTimeout);
            }
            if (terminal) {
                terminal.dispose();
            }
        };
    });

    function getPrompt(): string {
        const dir = currentCwd.split(/[\/\\]/).pop() || '';
        return `${user}@${host} ${dir} % `;
    }

    function joinPaths(base: string, relative: string): string {
        const sep = base.includes('\\') ? '\\' : '/';
        if (relative.match(/^[a-zA-Z]:/) || relative.startsWith('/')) {
            return relative;
        }
        let parts = base.split(sep).filter((p) => p !== '');
        relative.split(sep).forEach((part) => {
            if (part === '' || part === '.') return;
            if (part === '..') {
                if (parts.length > 0) parts.pop();
                return;
            }
            parts.push(part);
        });
        let newPath = parts.join(sep);
        if (base.startsWith(sep) || base.match(/^[a-zA-Z]:/)) {
            newPath = (base.startsWith(sep) ? sep : '') + newPath;
        }
        return newPath;
    }

    async function handleCdCommand(command: string) {
        let target = command.slice(3).trim();
        let newCwd: string;
        if (target === '' || target === '~') {
            newCwd = home;
        } else if (target.startsWith('~/')) {
            newCwd = joinPaths(home, target.slice(2));
        } else {
            newCwd = joinPaths(currentCwd, target);
        }
        try {
            const isDir: boolean = await invoke('is_directory', { path: newCwd });
            if (isDir) {
                currentCwd = newCwd;
                terminal?.write(getPrompt());
            } else {
                terminal?.write(`No such directory: ${newCwd}\r\n${getPrompt()}`);
            }
        } catch (error) {
            terminal?.write(`Error: ${error}\r\n${getPrompt()}`);
            terminal?.scrollToBottom();
        }
        terminal?.scrollToBottom();
    }

    function updateCommandLine(newCommand: string) {
        const currentPrompt = getPrompt();
        const eraseLength = currentPrompt.length + commandBuffer.length;
        const eraseStr = ' '.repeat(eraseLength);
        terminal?.write('\r' + eraseStr + '\r' + currentPrompt + newCommand);
        commandBuffer = newCommand;
        terminal?.scrollToBottom();
    }

    function handleWindowResize() {
        if (isTerminalOpen && fitAddon) {
            fitAddon.fit();
            terminal?.scrollToBottom();
        }
        syncLineNumbersScroll();
    }

    function toggleTerminal() {
        isTerminalOpen = !isTerminalOpen;
        if (isTerminalOpen && terminal && fitAddon) {
            fitAddon.fit();
            terminal.focus();
            terminal.scrollToBottom();
        }
    }

    function handleTerminalResize(event: MouseEvent) {
        const startY = event.clientY;
        const startHeight = terminalHeight;
        const minHeight = 100;
        const maxHeight = window.innerHeight * 0.5;

        function onMouseMove(moveEvent: MouseEvent) {
            const deltaY = startY - moveEvent.clientY;
            const newHeight = startHeight + deltaY;
            terminalHeight = Math.max(minHeight, Math.min(maxHeight, newHeight));
            if (fitAddon) {
                fitAddon.fit();
                terminal?.scrollToBottom();
            }
        }

        function onMouseUp() {
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', onMouseUp);
        }

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }

    function handleKeyDown(event: KeyboardEvent) {
        if ((event.ctrlKey || event.metaKey) && event.key === 's') {
            event.preventDefault();
            if (selectedFile && isEdited) {
                saveFile();
            }
        } else if (event.key === 'Escape') {
            window.location.href = "/";
        } else if (event.key !== 'Alt' && event.key !== 'Meta') {
            handleEditorChange(event);
        }
    }

    async function loadFiles(path: string, level: number = 0) {
        try {
            const dirFiles = await invoke("list_files", { dirPath: path }) as FileEntry[];
            dirFiles.sort((a, b) => {
                if (a.is_dir && !b.is_dir) return -1;
                if (!a.is_dir && b.is_dir) return 1;
                return a.name.localeCompare(b.name);
            });

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

    function handleEditorChange(event: Event) {
        const target = event.target as HTMLTextAreaElement;
        if (target === editorElement && target.value !== undefined) {
            editorContent = target.value;
            isEdited = editorContent !== fileContent;
            updateLineNumbers(editorContent);
            updateCurrentLine(event);

            if (isEdited && selectedFile) {
                if (autoSaveTimeout !== null) {
                    clearTimeout(autoSaveTimeout);
                }
                autoSaveTimeout = setTimeout(autoSave, 500);
            }
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
        lineCount = text.split('\n').length;
        syncLineNumbersScroll();
    }

    function syncLineNumbersScroll() {
        if (editorWrapper && editorElement) {
            const lineNumbersContent = editorWrapper.querySelector('.line-numbers-content') as HTMLDivElement;
            if (lineNumbersContent) {
                lineNumbersContent.style.transform = `translateY(-${editorElement.scrollTop}px)`;
                lineNumbersContent.style.height = `${editorElement.scrollHeight}px`;
            }
        }
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

    async function switchToFile(index: number) {
        if (index < 0 || index >= openFiles.length) return;
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
        }

        activeFileIndex = index;
        const file = openFiles[index];
        selectedFile = file.path;

        try {
            fileContent = await invoke("read_file", { path: file.path });
            editorContent = fileContent;
            isEdited = false;
            updateLineNumbers(fileContent);
            if (editorElement) {
                editorElement.value = editorContent;
            }
        } catch (error) {
            console.error("Error reading file:", error);
            fileContent = "Cannot display contents of the file";
            editorContent = fileContent;
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

    function toggleSidebar() {
        sidebarWidth = isSidebarOpen ? 0 : 300;
        isSidebarOpen = sidebarWidth > 0;
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
    <div class="sidebar--tools">
        <button class="sidebar--tools-item" class:active={isSidebarOpen} on:click={toggleSidebar}>
            <Folder size={25} />
        </button>
        <button class="sidebar--tools-item bottom" class:active={isTerminalOpen} on:click={toggleTerminal}>
            <TerminalIcon size={25} />
        </button>
    </div>
    <div class="sidebar" style="width: {sidebarWidth}px">
        {#if projectPath}
            <div class="file-list">
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
    <div class="resizer" on:mousedown={handleResize}></div>

    <div class="editor-area" style="width: calc(100% - {sidebarWidth}px - 5px); height: {isTerminalOpen ? `calc(100vh - ${terminalHeight}px - 25px)` : 'calc(100vh - 25px)'};">
        <div class="editor-header" class:editor-header--closed={openFiles.length === 0}>
            {#each openFiles as file, index}
                <div
                        class="file-tab"
                        class:active={index === activeFileIndex}
                        on:click={() => switchToFile(index)}
                >
                    <span class="file-tab-name">{getDisplayName(file)}</span>
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
                <div class="code-editor--highlight"></div>
                <textarea
                        class={`code-editor ${fileContent === "Cannot display contents of the file" ? 'no-file-selected' : ''}`}
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

    <div class="terminal-container" style="display: {isTerminalOpen ? 'block' : 'none'}; height: {isTerminalOpen ? `${terminalHeight}px` : '0'};">
        <div class="terminal-resizer" on:mousedown={handleTerminalResize}></div>
        <div class="terminal-wrapper">
            <div class="terminal" bind:this={terminalElement}></div>
        </div>
    </div>
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