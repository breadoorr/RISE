<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from "@tauri-apps/api/core";
    import { Terminal } from 'xterm';
    import { FitAddon } from 'xterm-addon-fit';
    import { WebLinksAddon } from 'xterm-addon-web-links';
    import 'xterm/css/xterm.css';
    import { ChevronDown } from "lucide-svelte";

    export let isTerminalOpen: boolean;
    export let terminalHeight: number;
    export let projectPath: string | null;
    export let user: string;
    export let host: string;
    export let home: string;
    export let toggleTerminal: () => void;

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
    let selectedShell: string = localStorage.getItem('terminalShell') || 'system';
    let shellMenuOpen: boolean = false;
    let defaultShellId: string = 'sh';

    const shellOptions: { id: string; label: string; os: 'unix' | 'mac' | 'win' | 'any' }[] = [
        { id: 'zsh', label: 'zsh', os: 'mac' },
        { id: 'bash', label: 'bash', os: 'unix' },
        { id: 'sh', label: 'sh', os: 'unix' },
        { id: 'cmd', label: 'cmd', os: 'win' },
        { id: 'powershell', label: 'PowerShell', os: 'win' },
    ];

    const isWindows = navigator.userAgent.toLowerCase().includes('windows');
    const isMac = navigator.userAgent.toLowerCase().includes('mac');

    function getCssVar(name: string): string {
        // Prefer variables defined on body (theme classes are applied to body), fallback to :root
        const bodyVal = getComputedStyle(document.body).getPropertyValue(name);
        if (bodyVal && bodyVal.trim()) return bodyVal.trim();
        const rootVal = getComputedStyle(document.documentElement).getPropertyValue(name);
        return rootVal ? rootVal.trim() : '';
    }

    function getXtermTheme(): { cursor: string; background: string; foreground: string } {
        const cursor = getCssVar('--text-950');
        const background = getCssVar('--background-100');
        const foreground = getCssVar('--text-800');
        return {
            cursor: cursor,
            background: background,
            foreground: foreground,
        };
    }

    function applyThemeToAllTerminals() {
        const theme = getXtermTheme();
        terminalTabs.forEach(t => {
            if (!t.terminal) return;
            // Use runtime option update to ensure xterm applies new theme immediately
            t.terminal.options.theme = {
                cursor: theme.cursor,
                background: theme.background,
                foreground: theme.foreground,
            };
        });
    }

    let themeObserver: MutationObserver | null = null;

    function nextTabTitleForShell(shellId: string): string {
        const existingCount = terminalTabs.filter(t => t.shellId === shellId).length;
        if (existingCount === 0) return `${shellId}`;
        return `${shellId} (${existingCount})`;
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
        setTimeout(() => { tab.fitAddon?.fit(); term.scrollToBottom(); }, 0);
    }

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

    function handleWindowResize() {
        const tab = getActiveTab();
        if (isTerminalOpen && tab?.fitAddon) {
            if (terminalHeight > window.innerHeight * 0.7) {
                terminalHeight = window.innerHeight * 0.7;
            }
            tab.fitAddon.fit();
            tab.terminal?.scrollToBottom();
        }
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
                activeTerminalTabId = null;
                isTerminalOpen = false;
            }
        } else if (terminalTabs.length === 0) {
            activeTerminalTabId = null;
            isTerminalOpen = false;
        }
    }

    onMount(async () => {
        try {
            defaultShellId = await invoke('get_default_shell') as string;
        } catch (e) {
            console.log(e);
        }
        const stored = localStorage.getItem('terminalShell');
        if (isWindows) {
            selectedShell = (stored && ['cmd','powershell'].includes(stored)) ? stored : defaultShellId;
        } else {
            selectedShell = (stored && ['zsh','bash','sh'].includes(stored)) ? stored : defaultShellId;
        }
        const mql = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)');
        const onSchemeChange = () => applyThemeToAllTerminals();
        if (mql && 'addEventListener' in mql) {
            mql.addEventListener('change', onSchemeChange);
        }
        themeObserver = new MutationObserver(() => applyThemeToAllTerminals());
        // Observe theme class changes; theme classes are applied to body
        const target1 = document.body;
        const target2 = document.documentElement;
        if (target1) themeObserver.observe(target1, { attributes: true, attributeFilter: ['class', 'data-theme', 'style'] });
        if (target2 && target2 !== target1) themeObserver.observe(target2, { attributes: true, attributeFilter: ['class', 'data-theme', 'style'] });

        window.addEventListener('resize', handleWindowResize);

        toggleTerminal = () => {
            isTerminalOpen = !isTerminalOpen;
            if (isTerminalOpen && terminalTabs.length === 0) {
                createTerminalTab(defaultShellId);
            } else if (isTerminalOpen) {
                const tab = getActiveTab();
                if (tab?.fitAddon && tab?.terminal) {
                    tab.fitAddon.fit();
                    tab.terminal.focus();
                    tab.terminal.scrollToBottom();
                }
            }
        };

        // Create initial terminal tab if terminal is open
        if (isTerminalOpen && terminalTabs.length === 0) {
            createTerminalTab(selectedShell || defaultShellId);
        }

        return () => {
            window.removeEventListener('resize', handleWindowResize);
            terminalTabs.forEach(t => t.terminal?.dispose());
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
</script>

<div class="terminal-container" style="display: {isTerminalOpen ? 'flex' : 'none'}; height: {isTerminalOpen ? `${terminalHeight}px` : '0'};">
    <button aria-label="terminal-resizer" class="terminal-resizer" on:mousedown={handleTerminalResize}></button>
    <div class="terminal-toolbar">
        <div class="terminal-toolbar-left">
            <span class="terminal-title">Terminal</span>
        </div>
        <div class="terminal-tabs">
            {#each terminalTabs as t}
                <div class="term-tab" class:active={t.id === activeTerminalTabId}>
                    <button class="term-tab--title-button" on:click={() => switchToTerminalTab(t.id)}>
                        <span>{t.title}</span>
                    </button>
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
                    <div class="shell-menu">
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
    <div class="terminal-panes" style="height: calc({terminalHeight}px - 5%)">
        {#each terminalTabs as t (t.id)}
            <div class="terminal-pane" style="display: {t.id === activeTerminalTabId ? 'block' : 'none'};">
                <div class="terminal" id={"term-" + t.id} bind:this={t.element}></div>
            </div>
        {/each}
    </div>
</div>

<style lang="scss">
  @use '../style/terminal';
</style>