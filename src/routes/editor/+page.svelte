<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import Terminal from './components/Terminal.svelte';
    import Sidebar from "./components/Sidebar.svelte";
    import Editor from "./components/Editor.svelte";
    import Menu from "./components/Menu.svelte";
    import ProjectSearchModal from "./components/ProjectSearchModal.svelte";
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

    let PROJECTS: [string, string];

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
    let currentTheme: string;

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
        currentTheme = localStorage.getItem('theme') || 'default';
        document.body.classList.toggle(currentTheme + '-theme');
        projectPath = localStorage.getItem('projectPath');
        if (projectPath) {
            // Inform backend that the project is opened so watcher starts
            try { await invoke('open_project', { path: projectPath }); } catch (e) { console.error('open_project failed', e); }

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

        PROJECTS = await invoke('get_recent_projects');

        // Editor component now handles keydown/focus; keep click/keyup here only for FileMenu interactions
        window.addEventListener('keyup', handleInputEvent);
        window.addEventListener('click', handleInputEvent);

        // Listen for backend file system changes and refresh current project tree
        const unlistenFs = await listen<string>('fs-changed', async (_evt) => {
            try {
                const state = get(fileStore) as { projectPath: string | null };
                if (state.projectPath) {
                    await refreshPathInStore(state.projectPath);
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
  // Project-wide search modal state
  let showProjectSearch = false;
  let projQuery = '';
  let projReplacement = '';
  let projCaseSensitive = false;
  let projRegex = false;
  let projResults: { path: string; line: number; column: number; line_text: string }[] = [];
  let projPathResults: { path: string; name: string; is_dir: boolean }[] = [];
  let projBusy = false;
  let lastShiftTime = 0;
  let searchNames = false;
  let includeFiles = true;
  let includeDirs = true;

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if (e.key === 'Shift') {
      const now = Date.now();
      if (now - lastShiftTime < 350) {
        // double Shift detected
        e.preventDefault();
        showProjectSearch = true;
        setTimeout(() => {
          const input = document.querySelector('.project-search-modal input.query') as HTMLInputElement | null;
          if (input) { input.focus(); input.select(); }
        }, 0);
      }
      lastShiftTime = now;
    }
    if (showProjectSearch && e.key === 'Escape') {
      showProjectSearch = false;
    }
  }

  async function runProjectSearch() {
    if (!projectPath || !projQuery) { projResults = []; projPathResults = []; return; }
    projBusy = true;
    try {
      const res = await invoke('search_in_project', { rootPath: projectPath, query: projQuery, caseSensitive: projCaseSensitive, regex: projRegex, maxResults: 5000 }) as any[];
      // Map snake_case from Rust to camelCase fields used here
      projResults = res.map((r: any) => ({ path: r.path, line: r.line, column: r.column, line_text: r.line_text }));

      if (searchNames) {
        try {
          const pres = await invoke('search_paths_in_project', {
            rootPath: projectPath,
            query: projQuery,
            caseSensitive: projCaseSensitive,
            includeDirs,
            includeFiles,
            maxResults: 5000
          }) as any[];
          projPathResults = pres.map((p: any) => ({ path: p.path, name: p.name, is_dir: p.is_dir }));
        } catch (e) {
          console.error('search_paths_in_project failed', e);
          projPathResults = [];
        }
      } else {
        projPathResults = [];
      }
    } catch (e) {
      console.error('search_in_project failed', e);
      projResults = [];
      projPathResults = [];
    } finally {
      projBusy = false;
    }
  }

  async function runProjectReplace(dryRun = false) {
    if (!projectPath || !projQuery) return;
    const ok = confirm(dryRun ? 'Preview replacements?' : 'Replace across project?');
    if (!ok) return;
    projBusy = true;
    try {
      const changed = await invoke('replace_in_project', { rootPath: projectPath, query: projQuery, replacement: projReplacement, caseSensitive: projCaseSensitive, regex: projRegex, dryRun, maxFiles: 500 }) as number;
      if (!dryRun) {
        // refresh tree
        try { await refreshPathInStore(projectPath); } catch {}
        // rerun search to update preview
        await runProjectSearch();
      } else {
        alert(`Would change ${changed} file(s).`);
      }
    } catch (e) {
      console.error('replace_in_project failed', e);
    } finally {
      projBusy = false;
    }
  }

  function projectModalKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') { showProjectSearch = false; e.stopPropagation(); }
  }
  function projectQueryKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') { runProjectSearch(); }
    if (e.key === 'Escape') { showProjectSearch = false; e.stopPropagation(); }
  }

  async function openSearchResult(item: { path: string; line: number; column: number }) {
    // open or focus file tab, then set caret to location
    const entry: FileEntry = { path: item.path, name: await basename(item.path), is_dir: false, level: 0, parent_dir: '', children: undefined } as any;
    openFileFromSidebar(entry);
    // Wait a tick for buffer to load
    setTimeout(() => {
      if (!editorElement || editorContent.length === 0) return;
      // compute index from line/column (1-based)
      let idx = 0; let line = 1; let i = 0;
      while (i < editorContent.length && line < item.line) {
        const ch = editorContent[i++];
        idx++;
        if (ch === '\n') line++;
      }
      idx += Math.max(0, item.column - 1);
      editorElement.selectionStart = idx;
      editorElement.selectionEnd = idx;
      editorElement.focus();
    }, 50);
  }

  async function expandFolderInSidebar(folderPath: string) {
    if (!projectPath) return;
    // Normalize and break path into segments
    const root = projectPath;
    if (!folderPath.startsWith(root)) {
      // fallback: just refresh store at folderPath
      await refreshPathInStore(folderPath);
      return;
    }
    const rel = folderPath.slice(root.length).replace(/^\/+/, '');
    const parts = rel.length ? rel.split('/') : [];

    // Iteratively expand down the tree and refresh
    let current = root;
    for (const part of parts) {
      current = current.endsWith('/') ? current + part : current + '/' + part;
      // Mark node expanded if it exists in current local tree
      fileStore.update((state) => {
        const mutate = (node: any): boolean => {
          if (!node) return false;
          if (node.path === current) {
            if (node.is_dir) node.expanded = true;
            return true;
          }
          if (node.children) {
            for (const c of node.children) {
              if (mutate(c)) return true;
            }
          }
          return false;
        };
        const nextFiles = [...state.files];
        if (nextFiles[0]) mutate(nextFiles[0]);
        return { ...state, files: nextFiles } as any;
      });
      await refreshPathInStore(current);
    }
  }

  async function openPathResult(item: { path: string; name: string; is_dir: boolean }) {
    if (item.is_dir) {
      await expandFolderInSidebar(item.path);
    } else {
      const entry: FileEntry = { path: item.path, name: await basename(item.path), is_dir: false, level: 0, parent_dir: '', children: undefined } as any;
      openFileFromSidebar(entry);
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  });
</script>

<Menu Actions={actions} x={x} y={y} isMenuOpen={isSettingOpen} triggerAction={(action) => {
    if (projects) {
        localStorage.setItem('projectPath', PROJECTS.find(p => p[1] === action)[0])
        window.location.href = "/editor";
    } else {
        if (action === "Theme") changeTheme();
    }
}} />

<div class="window-title">
    <button class="project-tab window-title--button" on:click={(e) => {
        isSettingOpen = !isSettingOpen;
        x = e.clientX;
        y = 30;
        actions = PROJECTS;
        actions = actions.map(p => p[1]);
        projects = true;
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

<ProjectSearchModal
  open={showProjectSearch}
  bind:query={projQuery}
  bind:replacement={projReplacement}
  bind:caseSensitive={projCaseSensitive}
  results={projResults}
  busy={projBusy}
  bind:searchNames
  bind:includeDirs
  bind:includeFiles
  pathResults={projPathResults}
  on:close={() => showProjectSearch = false}
  on:search={runProjectSearch}
  on:preview={() => runProjectReplace(true)}
  on:replace={() => runProjectReplace(false)}
  on:openResult={(e) => openSearchResult(e.detail)}
  on:openPath={(e) => openPathResult(e.detail)}
/>

<style lang="scss">
  @use 'style/main';
  .project-search-modal { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 10vh; z-index: 50; }
  .project-search-modal .modal-card { width: min(960px, 90vw); max-height: 80vh; background: #1e1e1e; color: #ddd; border: 1px solid #444; border-radius: 8px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); display: flex; flex-direction: column; }
  .project-search-modal .modal-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; border-bottom: 1px solid #333; }
  .project-search-modal .modal-header .close { background: transparent; border: none; color: #ddd; font-size: 20px; cursor: pointer; }
  .project-search-modal .modal-body { padding: 10px 12px; display: flex; flex-direction: column; gap: 8px; }
  .project-search-modal .controls { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .project-search-modal input { background: #222; color: #ddd; border: 1px solid #444; padding: 6px 8px; border-radius: 6px; }
  .project-search-modal button { background: #333; color: #ddd; border: 1px solid #555; padding: 6px 10px; border-radius: 6px; cursor: pointer; }
  .project-search-modal .results { overflow: auto; max-height: 60vh; border-top: 1px solid #333; }
  .project-search-modal ul { list-style: none; margin: 0; padding: 0; }
  .project-search-modal .result { padding: 8px 6px; border-bottom: 1px solid #2a2a2a; cursor: pointer; }
  .project-search-modal .result:hover { background: #2a2a2a; }
  .project-search-modal .result .path { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; opacity: 0.9; }
  .project-search-modal .result .loc { font-size: 12px; opacity: 0.8; }
  .project-search-modal .result .preview { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; white-space: pre; overflow: hidden; text-overflow: ellipsis; }
</style>