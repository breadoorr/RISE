<script lang="ts">
    import { invoke } from "@tauri-apps/api/tauri";
    import {fileStore, refreshPathInStore} from "$lib/stores/fileStore";

    // Props from Sidebar
    export let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;
    export let createNewItem: (isDir: boolean, parentPath: string, onNameConfirmed: (name: string) => Promise<void>) => void;

    let Actions: Array<string> = [];
    let projectPath = "";
    let currentPath: string = ""; // Renamed from Path to avoid confusion
    let isDir: boolean = false;
    let isMenuOpen = false;
    let x = 10;
    let y = 10;

    // Override the prop to handle menu positioning and actions
    function handleToggleFileMenu(event: Event, isContextMenu: boolean, is_dir: boolean = false, path: string = "", project_path: string | null = null) {
        const e = event as MouseEvent;
        isMenuOpen = isContextMenu;
        if (isMenuOpen) {
            x = e.clientX;
            y = e.clientY;
            isDir = is_dir;
            currentPath = path;
            if (project_path) projectPath = project_path;
            invoke("get_actions", { isDir }).then((actions) => {
                Actions = actions as Array<string>;
            }).catch(console.error);
        }
    }

    // Expose the handler to Sidebar via bind
    $: toggleFileMenu = handleToggleFileMenu;

    async function triggerAction(action: string) {
        if (action === "New File" || action === "New Folder") {
            isMenuOpen = false;
            const isDirAction = action === "New Folder";
            const parentPath = currentPath || projectPath;

            // Prepare the onNameConfirmed callback for the Sidebar
            const onNameConfirmed = async (name: string) => {
                try {
                    const newPath = `${parentPath}/${name}`;
                    await invoke("perform_action", {
                        action,
                        file: { path: newPath, name, is_dir: isDirAction }
                    });
                    console.log("Created item:", newPath);
                    await refreshPathInStore(parentPath);
                } catch (e) {
                    console.error("Failed to create item:", e);
                    throw e; // Let Sidebar handle UI cleanup
                }
            };

            // Call Sidebar's createNewItem with the callback
            createNewItem(isDirAction, parentPath, onNameConfirmed);
            // await refreshPathInStore(parentPath);
        }
        // Add handling for other actions (e.g., Delete) here if needed
    }
</script>

<div class="file-menu-container" style="display: {isMenuOpen ? 'flex' : 'none'}; left: {x}px; top: {y}px">
    {#if Actions.length > 0}
        {#each Actions as action}
            <button on:click={() => triggerAction(action)} class="file-menu-item">{action}</button>
        {/each}
    {/if}
</div>

<style lang="scss">
  @use '../style/file-menu';
</style>