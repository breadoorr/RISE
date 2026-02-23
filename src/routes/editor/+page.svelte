<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import Terminal from './components/Terminal.svelte';
    import Sidebar from "./components/Sidebar.svelte";
    import Editor from "./components/Editor.svelte";
    import Menu from "./components/Menu.svelte";
    import type { FileEntry } from '$lib/utils/types';
    import {fileStore, refreshPathInStore, setProjectInfo} from '$lib/stores/fileStore';
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
    import { get } from 'svelte/store';

    const SETTINGS: string[] = [
        "Theme",
        "Keymap",
        "View Mode"
    ];

    const THEMES: string[] = [
        "latte",
        "macchiato",
        "mocha"
    ]

    import type { ProjectEntry, RunConfig } from '$lib/utils/types';
    let PROJECTS: ProjectEntry[] = [];

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
    let projects: boolean = false;
    let runMenu: boolean = false;
    let currentRunConfigId: string | null = null;
    let currentRunConfigName: string = '';
    let runActions: string[] = [];
    let runNameToId: Record<string, string> = {};
    let currentTheme: string;

    // user/system info
    let user: string = '';
    let host: string = '';
    let home: string = '';

    // Reference to toggleTerminal function from Terminal component
    let toggleTerminal: () => void;
    let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;
    let runInTerminalFn: ((command: string, cwd?: string, shellId?: string) => Promise<void>) | null = null;

    function prepareRunActions() {
        try {
            const state = get(fileStore) as any;
            const proj = state?.project as { run_configs?: RunConfig[] } | undefined;
            runActions = [];
            runNameToId = {};
            if (proj?.run_configs) {
                for (const rc of proj.run_configs) {
                    runActions.push(rc.name);
                    runNameToId[rc.name] = rc.id;
                }
            }
        } catch {}
    }

    async function openRunMenu(e: MouseEvent) {
        runMenu = !runMenu;
        isSettingOpen = false;
        prepareRunActions();
        x = e.clientX;
        y = 30;
    }

    async function applySelectedRunByName(name: string) {
        const id = runNameToId[name] || name;
        if (!projectPath) return;
        try {
            await invoke('set_selected_run_config', { path: projectPath, runConfigId: id });
            currentRunConfigId = id;
            currentRunConfigName = name;
        } catch (e) {
            console.error('Failed to set selected run config', e);
        }
    }

    function getActiveFilePath(): string | null {
        if (activeFileIndex >= 0 && activeFileIndex < openFiles.length) {
            return openFiles[activeFileIndex]?.path || null;
        }
        return selectedFile || null;
    }

    function buildRunCommandForCurrentFile(): { cmd: string, cwd: string } | null {
        const filePath = getActiveFilePath();
        if (!filePath || !projectPath) return null;
        const lower = filePath.toLowerCase();
        const quote = (s: string) => s.includes(' ') ? `"${s}"` : s;
        if (lower.endsWith('.js')) return { cmd: `node ${quote(filePath)}`, cwd: projectPath };
        if (lower.endsWith('.ts')) return { cmd: `npx ts-node ${quote(filePath)}`, cwd: projectPath };
        if (lower.endsWith('.py')) return { cmd: `python3 ${quote(filePath)}`, cwd: projectPath };
        if (lower.endsWith('.sh')) return { cmd: `bash ${quote(filePath)}`, cwd: projectPath };
        if (lower.endsWith('.rs')) return { cmd: `cargo run`, cwd: projectPath };
        if (lower.endsWith('.mjs')) return { cmd: `node ${quote(filePath)}`, cwd: projectPath };
        if (lower.endsWith('.cjs')) return { cmd: `node ${quote(filePath)}`, cwd: projectPath };
        return { cmd: `echo Unsupported file type for run: ${quote(filePath)}`, cwd: projectPath };
    }

    async function handleRunClick() {
        try {
            if (!runInTerminalFn) return;
            const stateAny: any = get(fileStore);
            const proj = stateAny?.project as { run_configs?: RunConfig[], path?: string, project_type?: string } | undefined;
            const rcs = proj?.run_configs || [];
            let chosen = rcs.find((r) => r.id === currentRunConfigId);
            if (!chosen && rcs.length > 0) {
                chosen = rcs[0];
                currentRunConfigId = chosen.id;
                currentRunConfigName = chosen.name;
            }
            if (!chosen) return;
            let cmdToRun = chosen.command;
            let cwd = chosen.cwd || proj?.path || projectPath || '';
            if (cmdToRun === 'run_current_file') {
                const built = buildRunCommandForCurrentFile();
                if (!built) return;
                cmdToRun = built.cmd;
                cwd = built.cwd;
            }
            await runInTerminalFn(cmdToRun, cwd);
        } catch (e) {
            console.error('Run failed', e);
        }
    }

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
        currentTheme = localStorage.getItem('theme') || 'default';
        document.body.classList.toggle(currentTheme + '-theme');
        projectPath = localStorage.getItem('projectPath');
        if (projectPath) {
            // Inform backend that the project is opened so watcher starts
            try { await invoke('open_project', { path: projectPath }); } catch (e) { console.error('open_project failed', e); }

            // Fetch enriched project info (name, type, run configs)
            let info: any = null;
            try { info = await invoke('get_project_info', { path: projectPath }); } catch (e) { console.error('get_project_info failed', e); }

            currentPath = projectPath;
            if (info && info.name && info.path) {
                setProjectInfo(info);
                projectName = info.name;
                projectPath = info.path;
                try {
                    const sel: string | null = await invoke('get_selected_run_config', { path: projectPath });
                    const rcs: any[] = Array.isArray(info.run_configs) ? info.run_configs : [];
                    if (sel) {
                        const found = rcs.find((r) => r.id === sel);
                        currentRunConfigId = sel;
                        currentRunConfigName = found ? found.name : '';
                    } else if (rcs.length > 0) {
                        currentRunConfigId = rcs[0].id;
                        currentRunConfigName = rcs[0].name;
                    }
                } catch (e) { console.error('get_selected_run_config failed', e); }
            } else {
                projectName = await basename(projectPath);
            }
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

        PROJECTS = await invoke('get_recent_projects');

        // Editor component now handles keydown/focus; keep click/keyup here only for FileMenu interactions
        window.addEventListener('keyup', handleInputEvent);
        window.addEventListener('click', handleInputEvent);

        // Listen for backend file system changes and refresh current project tree
        const unlistenFs = await listen<string>('fs-changed', async (_evt) => {
            try {
                const state = get(fileStore) as { projectPath: string | null };
                if (state.projectPath) {
                    // Refresh file tree
                    await refreshPathInStore(state.projectPath);
                    // Refresh project info (name/type/run configs) so run menu reflects package.json/Cargo.toml changes
                    try {
                        const info: any = await invoke('get_project_info', { path: state.projectPath });
                        if (info && info.name && info.path) {
                            setProjectInfo(info);
                            // Update current run config label/id if selection changed or became invalid
                            const rcs: any[] = Array.isArray(info.run_configs) ? info.run_configs : [];
                            try {
                                const sel: string | null = await invoke('get_selected_run_config', { path: state.projectPath });
                                if (sel && rcs.find((r) => r.id === sel)) {
                                    currentRunConfigId = sel;
                                    const found = rcs.find((r) => r.id === sel);
                                    currentRunConfigName = found ? found.name : '';
                                } else if (rcs.length > 0) {
                                    // Fallback to the first available run config without persisting immediately
                                    currentRunConfigId = rcs[0].id;
                                    currentRunConfigName = rcs[0].name;
                                } else {
                                    currentRunConfigId = null;
                                    currentRunConfigName = '';
                                }
                            } catch (e) {
                                // If backend selection unavailable, fallback to first
                                if (Array.isArray(info.run_configs) && info.run_configs.length > 0) {
                                    currentRunConfigId = info.run_configs[0].id;
                                    currentRunConfigName = info.run_configs[0].name;
                                } else {
                                    currentRunConfigId = null;
                                    currentRunConfigName = '';
                                }
                            }
                        }
                    } catch (e) {
                        console.error('get_project_info on fs-changed failed', e);
                    }
                }
            } catch (e) {
                console.error('Failed to handle fs-changed event', e);
            }
        });

        return () => {
            window.removeEventListener('keyup', handleInputEvent);
            window.removeEventListener('click', handleInputEvent);
            try { unlistenFs(); } catch {}
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

    async function changeTheme() {
        let newTheme = "";
        if (currentTheme === "default") newTheme = THEMES[0];
        else if (currentTheme === THEMES[0]) newTheme = THEMES[1];
        else if (currentTheme === THEMES[1]) newTheme = THEMES[2];
        else if (currentTheme === THEMES[2]) newTheme = "default";

        console.log(newTheme);

        await invoke("update_app_theme", {newTheme: newTheme});
        localStorage.setItem("theme", newTheme)
        document.body.classList.replace(currentTheme + '-theme', newTheme + '-theme');
        currentTheme = newTheme;
    }

</script>

<Menu Actions={actions} x={x} y={y} isMenuOpen={isSettingOpen} triggerAction={(action) => {
    if (projects) {
        const target = PROJECTS.find(p => p.name === action);
        if (target) {
            localStorage.setItem('projectPath', target.path)
            window.location.href = "/editor";
        }
    } else {
        if (action === "Theme") changeTheme();
    }
}} />

<!-- Run config selector menu -->
<Menu Actions={runActions} x={x} y={y} isMenuOpen={runMenu} triggerAction={async (action) => {
    await applySelectedRunByName(action);
    runMenu = false;
}} />

<div class="window-title">
    <button class="project-tab window-title--button" on:click={(e) => {
        isSettingOpen = !isSettingOpen;
        x = e.clientX;
        y = 30;
        actions = PROJECTS.map(p => p.name);
        projects = true;
    }}>
        {projectName ?? 'Untitled'}
        <ChevronDown class="chevron-down" size={20} />
    </button>
    <div class="window-title--group">
    <button class="window-title--button">
        <Hammer size={20} />
    </button>
    <button class="window-title--button" on:click={handleRunClick}>
        <Play size={20} />
    </button>
    <button class="window-title--button" on:click={openRunMenu}>
        {currentRunConfigName || 'Select run config'}
        <ChevronDown class="chevron-down" size={16} />
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

    <Terminal bind:isTerminalOpen bind:terminalHeight {projectPath} {user} {host} {home} bind:toggleTerminal exposeRun={(fn) => { runInTerminalFn = fn; }} />

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