<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import {refreshPathInStore} from "$lib/stores/fileStore";
    import Menu from "./Menu.svelte";

    // Props from Sidebar
    export let toggleFileMenu: (event: Event, isContextMenu: boolean, isDir: boolean, path: string, currentPath: string | null) => void;
    export let createNewItem: (isDir: boolean, parentPath: string, onNameConfirmed: (name: string) => Promise<void>) => void;

    export let editItem: (isDir: boolean, parentPath: string, onNameConfirmed: (name: string) => Promise<void>) => void;
    export let moveItem: (path: string, newPath: string) => void;

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

    $: toggleFileMenu = handleToggleFileMenu;

    async function triggerAction(action: string) {
        if (action === "New File" || action === "New Folder") {
            isMenuOpen = false;
            const isDirAction = action === "New Folder";
            const parentPath = currentPath || projectPath;

            let onNameCreateConfirmed = async (name: string) => {
                try {
                    const newPath = `${parentPath}/${name}`;
                    await invoke("perform_action", {
                        action,
                        file: { path: newPath, name, is_dir: isDirAction },
                        newName: newPath
                    });
                    console.log("Created item:", newPath);
                    await refreshPathInStore(parentPath);
                } catch (e) {
                    console.error("Failed to create item:", e);
                    throw e;
                }
            };

            createNewItem(isDirAction, parentPath, onNameCreateConfirmed);
        } else if (action === "Rename File" || action === "Rename Folder") {
            isMenuOpen = false;
            const isDirAction = action === "Rename Folder";
            const parentPath = currentPath || projectPath;
            let onNameRenameConfirmed = async (name: string) => {
                try {
                    const newPath = parentPath.split("/").slice(0, -1).join('/').concat(`/${name}`);
                    console.log("New path:", newPath, " Parent path:", parentPath, "  Name:", name);
                    await invoke("perform_action", {
                        action,
                        file: { path: parentPath, name, is_dir: isDirAction },
                        newName: newPath
                    });
                    await refreshPathInStore(parentPath);
                } catch (e) {
                    console.error("Failed to create item:", e);
                    throw e;
                }
            };

            editItem(isDirAction, parentPath, onNameRenameConfirmed);
        } else {
            await invoke("perform_action", { action, file: { path: currentPath, name: "", is_dir: isDir }, newName: "" });
            await refreshPathInStore(currentPath);
        }
    }
</script>

<Menu Actions={Actions} x={x} y={y} isMenuOpen={isMenuOpen} triggerAction={triggerAction} />