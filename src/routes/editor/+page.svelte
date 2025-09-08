<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/tauri";
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
    // Multitab terminal state
    type TerminalTab = {
        id: string;
        title: string;
        shellId: string;
        terminal: Terminal | null;
        fitAddon: FitAddon | null;
        element: HTMLDivElement | null;
        commandBuffer: string;
        history: string[];
        historyIndex: number;
        cwd: string;
    };
    let terminalTabs: TerminalTab[] = [];
    let activeTerminalTabId: string | null = null;

    // Theme utils for xterm
    function getCssVar(name: string): string {
        const v = getComputedStyle(document.documentElement).getPropertyValue(name);
        return v ? v.trim() : '';
    }
    function getXtermTheme(): { background: string; foreground: string } {
        const background = getCssVar('--background-50');
        const foreground = getCssVar('--text-800');
        // Fallbacks in case variables are missing
        return {
            background: background || '#111111',
            foreground: foreground || '#e5e7eb',
        };
    }
    function applyThemeToAllTerminals() {
        const theme = getXtermTheme();
        terminalTabs.forEach(t => {
            t.terminal?.setOption('theme', theme as any);
        });
    }
    let themeObserver: MutationObserver | null = null;

    // user/system info
    let user: string = '';
    let host: string = '';
    let home: string = '';

    // Terminal shell selection
    let selectedShell: string = localStorage.getItem('terminalShell') || 'system';
    let shellMenuOpen: boolean = false;
    let defaultShellId: string = 'sh';
    const shellOptions: { id: string; label: string; os: 'unix' | 'mac' | 'win' | 'any' }[] = [
        { id: 'zsh', label: 'zsh', os: 'mac' },
        { id: 'bash', label: 'bash', os: 'unix', },
        { id: 'sh', label: 'sh', os: 'unix' },
        { id: 'cmd', label: 'Command Prompt (cmd)', os: 'win' },
        { id: 'powershell', label: 'PowerShell', os: 'win' },
    ];

    const isWindows = navigator.userAgent.toLowerCase().includes('windows');
        const isMac = navigator.userAgent.toLowerCase().includes('mac');
    function shellLabel(id: string): string {
        if (id === 'system') {
            const def = defaultShellId;
            const defLabel = shellOptions.find(o => o.id === def)?.label || def;
            return `System default (${defLabel})`;
        }
        return shellOptions.find(o => o.id === id)?.label || id;
    }

    function makeTabTitle(shellId: string): string {
        return `${shellId}`;
    }

    function nextTabTitleForShell(shellId: string): string {
        const base = makeTabTitle(shellId);
        const existingCount = terminalTabs.filter(t => t.shellId === shellId).length;
        if (existingCount === 0) return base;
        return `${base} (${existingCount})`;
    }

    function getActiveTab(): TerminalTab | null {
        return terminalTabs.find(t => t.id === activeTerminalTabId) || null;
    }

    function createTerminalTab(shellId: string) {
        const id = `tab-${Date.now()}-${Math.floor(Math.random()*10000)}`;
        const cwd = projectPath || home || '';
        const tab: TerminalTab = {
            id,
            title: nextTabTitleForShell(shellId),
            shellId,
            terminal: null,
            fitAddon: null,
            element: null,
            commandBuffer: '',
            history: [],
            historyIndex: -1,
            cwd,
        };
        terminalTabs = [...terminalTabs, tab];
        activeTerminalTabId = id;
        isTerminalOpen = true;
        // Initialize terminal after DOM binds element
        setTimeout(() => initTab(id), 0);
    }

    function initTab(id: string) {
        const tab = terminalTabs.find(t => t.id === id);
        if (!tab) return;
        if (!tab.element) {
            const el = document.getElementById('term-' + id) as HTMLDivElement | null;
            if (!el) { setTimeout(() => initTab(id), 0); return; }
            tab.element = el;
        }
        const term = new Terminal({
            theme: getXtermTheme() as any,
            fontFamily: 'monospace',
            lineHeight: 1.4,
            fontSize: 14,
            allowTransparency: false,
            convertEol: true,
            // rows: Math.floor(terminalHeight/1.6 / 14),
        });
        const fit = new FitAddon();
        term.loadAddon(fit);
        term.loadAddon(new WebLinksAddon());
        term.open(tab.element);
        term.write('Welcome to RISE IDE Terminal!\r\n' + getPromptFor(tab));
        term.scrollToBottom();

        term.onKey(async (event) => {
            const ev = event.domEvent;
            const key = ev.key;

            if (key === 'Enter') {
                term.write('\r\n');
                const command = tab.commandBuffer.trim();
                if (command) {
                    tab.history.push(command);
                    tab.historyIndex = tab.history.length;
                    if (command.startsWith('cd ')) {
                        await handleCdCommandFor(tab, command);
                    } else {
                        await invoke("execute_command_with_shell", { command, cwd: tab.cwd, shell: tab.shellId })
                            .then((result) => {
                                term.write(`${result}\r\n${getPromptFor(tab)}`);
                                term.scrollToBottom();
                            })
                            .catch(error => {
                                term.write(`${error}\r\n${getPromptFor(tab)}`);
                                term.scrollToBottom();
                            });
                    }
                } else {
                    term.write(getPromptFor(tab));
                    term.scrollToBottom();
                }
                tab.commandBuffer = '';
            } else if (key === 'Backspace') {
                if (tab.commandBuffer.length > 0) {
                    tab.commandBuffer = tab.commandBuffer.slice(0, -1);
                    term.write('\b \b');
                    term.scrollToBottom();
                }
            } else if (key === 'ArrowUp') {
                if (tab.historyIndex > 0) {
                    tab.historyIndex--;
                    updateCommandLineFor(tab, tab.history[tab.historyIndex]);
                    term.scrollToBottom();
                }
            } else if (key === 'ArrowDown') {
                if (tab.historyIndex < tab.history.length - 1) {
                    tab.historyIndex++;
                    updateCommandLineFor(tab, tab.history[tab.historyIndex]);
                } else if (tab.historyIndex === tab.history.length - 1) {
                    tab.historyIndex = tab.history.length;
                    updateCommandLineFor(tab, '');
                }
                term.scrollToBottom();
            } else if (ev.ctrlKey && key.toLowerCase() === 'l') {
                term.reset();
                term.write(getPromptFor(tab));
                tab.commandBuffer = '';
                term.scrollToBottom();
            } else if (!ev.ctrlKey && !ev.altKey && !ev.metaKey && key.length === 1 && key >= ' ' && key <= '~') {
                tab.commandBuffer += key;
                term.write(key);
                term.scrollToBottom();
            }
        });

        tab.terminal = term;
        tab.fitAddon = fit;
        // Fit after a tick
        setTimeout(() => { tab.fitAddon?.fit(); term.scrollToBottom(); }, 0);
    }

    function onShellChange(e: Event) {
        const value = (e.target as HTMLSelectElement).value;
        selectedShell = value;
        localStorage.setItem('terminalShell', selectedShell);
        // Open a new terminal tab with the selected shell
        createTerminalTab(selectedShell);
    }

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
        // initial cwd will be set per terminal tab

        try {
            defaultShellId = await invoke('get_default_shell') as string;
        } catch (e) {
            console.log(e);
        }
        // Determine initial shell selection
        const stored = localStorage.getItem('terminalShell');
        if (isWindows) {
            selectedShell = (stored && ['cmd','powershell'].includes(stored)) ? stored : defaultShellId;
        } else {
            selectedShell = (stored && ['zsh','bash','sh'].includes(stored)) ? stored : defaultShellId;
        }
        // Create initial terminal tab
        createTerminalTab(selectedShell);

        // Listen to system theme changes and re-apply xterm theme to all tabs
        const mql = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)');
        const onSchemeChange = () => applyThemeToAllTerminals();
        if (mql && 'addEventListener' in mql) {
            mql.addEventListener('change', onSchemeChange);
        } else if (mql && 'addListener' in mql) {
            // Safari/older browsers
            // @ts-ignore
            mql.addListener(onSchemeChange);
        }
        // Also observe class/attr changes on documentElement to catch app theme toggles
        themeObserver = new MutationObserver(() => applyThemeToAllTerminals());
        themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme', 'style'] });

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
            if (terminalTabs && terminalTabs.length) {
                terminalTabs.forEach(t => t.terminal?.dispose());
            }
            // Clean up theme listeners
            try {
                const mql = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)');
                const onSchemeChange = () => applyThemeToAllTerminals();
                if (mql && 'removeEventListener' in mql) {
                    mql.removeEventListener('change', onSchemeChange);
                } else if (mql && 'removeListener' in mql) {
                    // @ts-ignore
                    mql.removeListener(onSchemeChange);
                }
            } catch {}
            try { themeObserver?.disconnect(); } catch {}
        };
    });

    function getPromptFor(tab: TerminalTab): string {
        const dir = tab.cwd.split(/[\/\\]/).pop() || '';
        let shellText = `${user}@${host} ${dir} % `;
        if (tab.shellId === "bash") {
            shellText = `${host}:${dir} ${user}$ `;
        }
        return shellText;
    }

    async function handleCdCommandFor(tab: TerminalTab, command: string) {
        const target = command.slice(3);
        try {
            const newCwd = await invoke('change_directory', { cwd: tab.cwd, target });
            tab.cwd = newCwd as string;
            tab.terminal?.write(getPromptFor(tab));
        } catch (error) {
            tab.terminal?.write(`Error: ${error}\r\n${getPromptFor(tab)}`);
        }
        tab.terminal?.scrollToBottom();
    }

    function updateCommandLineFor(tab: TerminalTab, newCommand: string) {
        const currentPrompt = getPromptFor(tab);
        const eraseLength = currentPrompt.length + tab.commandBuffer.length;
        const eraseStr = ' '.repeat(eraseLength);
        tab.terminal?.write('\r' + eraseStr + '\r' + currentPrompt + newCommand);
        tab.commandBuffer = newCommand;
        tab.terminal?.scrollToBottom();
    }

    function handleWindowResize() {
        const tab = getActiveTab();
        if (isTerminalOpen && tab?.fitAddon) {
            tab.fitAddon.fit();
            tab.terminal?.scrollToBottom();
        }
        syncLineNumbersScroll();
    }

    function toggleTerminal() {
        isTerminalOpen = !isTerminalOpen;
        const tab = getActiveTab();
        if (isTerminalOpen) {
            if (terminalTabs.length === 0) {
                // Open default first tab when opening terminal with no tabs
                createTerminalTab(defaultShellId);
            } else if (tab?.fitAddon && tab?.terminal) {
                tab.fitAddon.fit();
                tab.terminal.focus();
                tab.terminal.scrollToBottom();
            }
        }
    }

    function handleTerminalResize(event: MouseEvent) {
        const startY = event.clientY;
        const startHeight = terminalHeight;
        const minHeight = 100;
        const maxHeight = window.innerHeight * 0.7;

        function onMouseMove(moveEvent: MouseEvent) {
            const deltaY = startY - moveEvent.clientY;
            const newHeight = startHeight + deltaY;
            terminalHeight = Math.max(minHeight, Math.min(maxHeight, newHeight));
            const tab = getActiveTab();
            if (tab?.fitAddon) {
                tab.fitAddon.fit();
                tab.terminal?.scrollToBottom();
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
            scheduleHighlight();

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

    function detectLanguageFromFilename(file: string | null): string {
        if (!file) return 'typescript';
        const f = file.toLowerCase();
        if (f.endsWith('.rs')) return 'rust';
        if (f.endsWith('.py')) return 'python';
        if (f.endsWith('.c') || f.endsWith('.h')) return 'c';
        if (f.endsWith('.java')) return 'java';
        if (f.endsWith('.cs')) return 'c_sharp';
        if (f.endsWith('.sql')) return 'sequel';
        // Use typescript grammar for ts/js by default
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
        const matches: number[] = []; // integrate find matches later
        const queryLen = 0;
        const path = selectedFile;
        highlightTimeout = setTimeout(async () => {
            try {
                const html = await invoke('highlight_html', { code: editorContent, language: lang, matches, queryLen, path }) as string;
                highlightHtml = html;
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
            scheduleHighlight();
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

    function switchToTerminalTab(id: string) {
        activeTerminalTabId = id;
        const tab = getActiveTab();
        setTimeout(() => { tab?.fitAddon?.fit(); tab?.terminal?.focus(); tab?.terminal?.scrollToBottom(); }, 0);
    }

    function closeTerminalTab(id: string, e?: MouseEvent) {
        if (e) e.stopPropagation();
        const idx = terminalTabs.findIndex(t => t.id === id);
        if (idx === -1) return;
        const closing = terminalTabs[idx];
        closing.terminal?.dispose();
        terminalTabs = terminalTabs.filter((t, i) => i !== idx);
        if (activeTerminalTabId === id) {
            if (terminalTabs.length > 0) {
                const newIdx = Math.max(0, idx - 1);
                activeTerminalTabId = terminalTabs[newIdx].id;
                setTimeout(() => {
                    const tab = getActiveTab();
                    tab?.fitAddon?.fit();
                    tab?.terminal?.focus();
                }, 0);
            } else {
                // When the last tab is closed, close the terminal UI
                activeTerminalTabId = null;
                isTerminalOpen = false;
            }
        } else if (terminalTabs.length === 0) {
            // If we closed a non-active tab and no tabs remain, also close the terminal UI
            activeTerminalTabId = null;
            isTerminalOpen = false;
        }
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
    <div class="resizer" on:mousedown={handleResize}></div>

    <div class="editor-area" style="width: calc(100% - {sidebarWidth}px - 5px); height: {isTerminalOpen ? `calc(100% - ${terminalHeight}px - 25px)` : 'calc(100vh - 25px)'};">
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

    <div class="terminal-container" style="display: {isTerminalOpen ? 'block' : 'none'}; height: {isTerminalOpen ? `${terminalHeight}px` : '0'};">
        <div class="terminal-resizer" on:mousedown={handleTerminalResize}></div>
        <div class="terminal-toolbar">
            <div class="terminal-toolbar-left">
                <span class="terminal-title">Terminal</span>
            </div>
            <div class="terminal-tabs">
                {#each terminalTabs as t}
                    <div class="term-tab" class:active={t.id === activeTerminalTabId} on:click={() => switchToTerminalTab(t.id)}>
                        <span>{t.title}</span>
                        <button class="term-tab-close" on:click={(e) => closeTerminalTab(t.id, e)} title="Close tab">×</button>
                    </div>
                {/each}
            </div>
            <div class="terminal-toolbar-right">
                <button class="term-tab-add" title="New tab" on:click={() => createTerminalTab(selectedShell)}>+</button>
                <div class="shell-menu-wrapper">
                    <button class="shell-menu-toggle" title="Open shell menu" on:click={() => shellMenuOpen = !shellMenuOpen}>
                        <ChevronDown size={15} />
                    </button>
                    {#if shellMenuOpen}
                        <div class="shell-menu" on:click={() => { /* prevent toolbar click bubbling */ }}>
                            {#each shellOptions as opt}
                                {#if isWindows}
                                    {#if opt.os === 'win'}
                                        <button class="shell-menu-item" on:click={() => { selectedShell = opt.id; localStorage.setItem('terminalShell', selectedShell); createTerminalTab(opt.id); shellMenuOpen = false; }}>
                                            {opt.label}
                                        </button>
                                    {/if}
                                {:else if isMac}
                                    {#if opt.id === 'zsh' || opt.id === 'bash'}
                                        <button class="shell-menu-item" on:click={() => { selectedShell = opt.id; localStorage.setItem('terminalShell', selectedShell); createTerminalTab(opt.id); shellMenuOpen = false; }}>
                                            {opt.label}
                                        </button>
                                    {/if}
                                {:else}
                                    {#if opt.id === 'bash' || opt.id === 'sh'}
                                        <button class="shell-menu-item" on:click={() => { selectedShell = opt.id; localStorage.setItem('terminalShell', selectedShell); createTerminalTab(opt.id); shellMenuOpen = false; }}>
                                            {opt.label}
                                        </button>
                                    {/if}
                                {/if}
                            {/each}
                        </div>
                    {/if}
                </div>
            </div>
        </div>
        <div class="terminal-panes" style="height: calc({terminalHeight}px - 70px)">
            {#each terminalTabs as t (t.id)}
                <div class="terminal-pane" style="display: {t.id === activeTerminalTabId ? 'block' : 'none'};">
                    <div class="terminal" id={"term-" + t.id} bind:this={t.element}></div>
                </div>
            {/each}
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