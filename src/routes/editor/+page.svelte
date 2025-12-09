<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import Terminal from './components/Terminal.svelte';
    import Sidebar from "./components/Sidebar.svelte";
    import Editor from "./components/Editor.svelte";
    import Menu from "./components/Menu.svelte";
    import type { FileEntry } from '$lib/utils/types';
    import {fileStore, refreshPathInStore} from '$lib/stores/fileStore';
    import { selectFile as selectFileInStore } from '$lib/stores/fileStore';
    import { loadFiles as loadFilesUtil, updateAllFiles as flattenFilesUtil } from '$lib/utils/fileLoader';
    import {
        BugIcon,
        ChevronDown,
        Hammer,
        HammerIcon,
        LucideHammer,
        Play,
        PlayIcon,
        PlaySquare,
        Settings
    } from "lucide-svelte";
    import { basename } from "@tauri-apps/api/path";

    const SETTINGS: string[] = [
        "Theme",
        "Keymap",
        "View Mode"
    ];

    const PROJECTS: string[] = [
        "DDD",
        "LOL"
    ]

    let projectPath: string | null = null;
    let projectName: string | null = null;
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
    let x: number, y: number;
    let isSettingOpen: boolean = false;
    let actions: string[] = [];

    // user/system info
    let user: string = '';
    let host: string = '';
    let home: string = '';

    // Reference to toggleTerminal function from Terminal component
    let toggleTerminal: () => void;
    let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;

    // React to centralized fileStore changes and update local state (useEffect-like)
    $: (function () {
        try {
            const state = $fileStore as { files: FileEntry[]; projectPath: string | null; selectedFile: FileEntry | null } | undefined;
            if (!state) return;
            function normalize(list: FileEntry[] | undefined): FileEntry[] {
                if (!list) return [];
                return list.map((f) => ({
                    expanded: f.is_dir ? (f.expanded ?? false) : undefined,
                    children: f.is_dir ? (f.children ?? []) : undefined,
                    level: f.level ?? 0,
                    parent_dir: f.parent_dir ?? '',
                    ...f,
                }));
            }

            const incoming = normalize(state.files);
            const shouldUpdate = incoming !== files || incoming.length !== files.length;
            if (shouldUpdate) {
                files = incoming;
                // Recompute flattened view without writing back to the store (avoid loops)
                const newAll: FileEntry[] = [];
                (function flatten(list: FileEntry[]) {
                    for (const f of list) {
                        newAll.push(f);
                        if (f.is_dir && f.expanded && f.children) {
                            flatten(f.children);
                        }
                    }
                })(files);
                allFiles = newAll;
            }
        } catch (e) {
            // Ignore if $fileStore not ready yet
        }
    })();

    onMount( async () => {
        projectPath = localStorage.getItem('projectPath');
        if (projectPath) {
            currentPath = projectPath;
            projectName = await basename(projectPath);
            const rootEntry: FileEntry = {
                path: projectPath,
                name: projectName || projectPath,
                is_dir: true,
                expanded: true,
                children: [],
                level: 0,
                parent_dir: ''
            };
            files = [rootEntry];
            allFiles = flattenFilesUtil(files);
            fileStore.update((state) => ({
                ...state,
                projectPath,
                files
            }));
            await refreshPathInStore(projectPath);
            // Sync file store with initial project and file tree
            fileStore.update((state) => ({
                ...state,
                projectPath,
                files
            }));
        }

        const info = await invoke('get_system_info') as { user: string; host: string; home: string };
        user = info.user;
        host = info.host;
        home = info.home;

        // Editor component now handles keydown/focus; keep click/keyup here only for FileMenu interactions
        window.addEventListener('keyup', handleInputEvent);
        window.addEventListener('click', handleInputEvent);

        return () => {
            window.removeEventListener('keyup', handleInputEvent);
            window.removeEventListener('click', handleInputEvent);
        };
    });

    // Sidebar now handles expand/collapse and loading. Parent only opens file tabs.
    function openFileFromSidebar(item: FileEntry) {
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
        }
        const existingIndex = openFiles.findIndex(file => file.path === item.path);
        if (existingIndex !== -1) {
            activeFileIndex = existingIndex;
            switchToFile(existingIndex);
        } else {
            openFiles = [...openFiles, item];
            activeFileIndex = openFiles.length - 1;
            switchToFile(activeFileIndex);
        }
        try { selectFileInStore(item); } catch {}
    }

    function handleInputEvent(event: Event) {
        // toggleFileMenu(event, false);
    }

    async function updateLineNumbers(text: string) {
        // handled inside Editor component
    }

    function scheduleHighlight() {
        // handled inside Editor component
    }

    async function switchToFile(index: number) {
        if (index < 0 || index >= openFiles.length) return;
        if (autoSaveTimeout !== null) {
            clearTimeout(autoSaveTimeout);
        }

        activeFileIndex = index;
        const file = openFiles[index];
        selectedFile = file.path;
        // Update store selected file
        try { selectFileInStore(file); } catch {}

        try {
            fileContent = await invoke("open_buffer", { path: file.path }) as string;
            editorContent = fileContent;
            lastBufferContent = fileContent;
            isEdited = false;
            await updateLineNumbers(fileContent);
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
            await updateLineNumbers(fileContent);
            if (editorElement) {
                editorElement.value = editorContent;
            }
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
                // Clear selection in store
                fileStore.update((state) => ({ ...state, selectedFile: null }));
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

</script>

<Menu Actions={actions} x={x} y={y} isMenuOpen={isSettingOpen} />

<div class="window-title">
    <button class="project-tab window-title--button" on:click={(e) => {
        isSettingOpen = !isSettingOpen;
        x = e.clientX;
        y = 30;
        actions = PROJECTS;
    }}>
        {projectName ?? 'Untitled'}
        <ChevronDown class="chevron-down" size={20} />
    </button>
    <div class="window-title--group">
    <button class="window-title--button">
        <Hammer size={20} />
    </button>
    <button class="window-title--button">
        <Play size={20} />
    </button>
    <button class="window-title--button">
        <BugIcon size={16} />
    </button>
    </div>

    <button class="window-title--button" on:click={(e) => {
        isSettingOpen = !isSettingOpen;
        x = e.clientX - 180;
        y = 30;
        actions = SETTINGS
    }}>
        <Settings size={20}/>
    </button>
</div>

<main>

    <Sidebar
        bind:files={files}
        bind:allFiles={allFiles}
        {projectPath}
        selectedFilePath={selectedFile}
        {sidebarWidth}
        {isSidebarOpen}
        {isTerminalOpen}
        {terminalHeight}
        on:toggleSidebar={(e) => {
            const w = e.detail?.width ?? (isSidebarOpen ? 0 : 300);
            sidebarWidth = w;
            isSidebarOpen = w > 0;
        }}
        on:toggleTerminal={() => toggleTerminal()}
        on:openFile={(e) => openFileFromSidebar(e.detail.file)}
        on:filesChanged={() => {
            fileStore.update((state) => ({
                ...state,
                projectPath,
                files
            }));
        }}
        on:resize={(e) => { sidebarWidth = e.detail.width; }}
        bind:toggleFileMenu
    />

    <Editor
        {sidebarWidth}
        {isTerminalOpen}
        {terminalHeight}
        {openFiles}
        {activeFileIndex}
        {selectedFile}
        bind:fileContent
        bind:editorContent
        bind:lineCount
        bind:currentLine
        bind:currentColumn
        bind:highlightHtml
        bind:isEdited
        setEditorRefs={({ editorElement: el, editorWrapper: wrap, highlightContainer: cont }) => {
            editorElement = el;
            editorWrapper = wrap;
            highlightContainer = cont;
        }}
        onTabClick={(index) => switchToFile(index)}
        onTabClose={(index, event) => closeFile(index, event)}
    />

    <Terminal bind:isTerminalOpen bind:terminalHeight {projectPath} {user} {host} {home} bind:toggleTerminal />

    <div class="editor-footer">
        <div class="cursor-info">
            <p class="cursor-info--line-number">Line: {currentLine}</p>
            <p class="cursor-info--column-number">Col: {currentColumn}</p>
        </div>
    </div>
</main>

<style lang="scss">
  @use 'style/main';
</style>