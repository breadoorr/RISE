<script lang="ts">
    import {invoke} from "@tauri-apps/api/tauri";
    import {refreshPathInStore} from "$lib/stores/fileStore";

    export let toggleFileMenu: (e: MouseEvent, open: boolean) => void;
    let Actions: Array<String> = [];
    let projectPath = "";
    let Path: string = "";
    let isDir: boolean = false;

    let isMenuOpen = false;

    let x = 10;
    let y = 10;

    toggleFileMenu = async (e: MouseEvent, open: boolean, is_dir: boolean = false, path: string = "", project_path: string = "") => {
        isMenuOpen = open;
        if (isMenuOpen) {
            x = e.clientX;
            y = e.clientY;

            isDir = is_dir;
            Actions = await invoke("get_actions", {isDir});
            if (project_path != "") projectPath = project_path;
            Path = path;
        }
    }

    export async function triggerAction(action: String) {
        await invoke("perform_action", {isDir, action, path: Path});
        let target = isDir ? (Path || projectPath) : (Path.split('/').slice(0, -1).join('/') || projectPath);
        if (action === "Delete" && isDir) {
            target = Path.split('/').slice(0, -1).join('/') || projectPath;
        }
        await refreshPathInStore(target || projectPath);
    }
</script>

<div class="file-menu-container" style="display: {isMenuOpen ? 'flex' : 'none'}; left: {x}px; top: {y}px">
    {#if Actions.length > 0}
        {#each Actions as action}
            <button onclick={() => triggerAction(action)} class="file-menu-item">{action}</button>
        {/each}
    {/if}
</div>

<style lang="scss">
  @use '../style/file-menu';
</style>