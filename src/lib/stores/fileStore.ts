import { writable } from 'svelte/store';
import type { FileEntry } from '$lib/utils/types';
import { invoke } from '@tauri-apps/api/core';

// Initialize the store with default values
export const fileStore = writable<{
    files: FileEntry[];
    projectPath: string | null;
    selectedFile: FileEntry | null;
}>({
    files: [],
    projectPath: null,
    selectedFile: null,
});

function makeRoot(projectPath: string, children: FileEntry[] = []): FileEntry {
    return {
        path: projectPath,
        name: projectPath.split('/').pop() || projectPath,
        is_dir: true,
        expanded: true,
        children,
        level: 0,
        parent_dir: ''
    };
}

// Function to load files and update the store (ensures root node exists)
export async function loadFilesIntoStore(projectPath: string | null) {
    if (!projectPath) return;
    try {
        const children: FileEntry[] = await invoke('list_files', { dirPath: projectPath });
        // Normalize children
        for (const f of children) {
            f.level = 1;
            f.parent_dir = projectPath || '';
            if (f.is_dir) {
                f.expanded = f.expanded ?? false;
                f.children = f.children ?? [];
            }
        }
        const root = makeRoot(projectPath, children);
        fileStore.update((state) => ({
            ...state,
            files: [root],
            projectPath,
        }));
    } catch (error) {
        console.error('Failed to load files:', error);
    }
}

// Incrementally refresh a directory or the parent of a file
export async function refreshPathInStore(path: string) {
    let currentProjectPath: string | null = null;
    let tree: FileEntry[] = [];
    const unsubscribe = fileStore.subscribe((s) => {
        currentProjectPath = s.projectPath;
        tree = s.files;
    });
    unsubscribe();
    if (!currentProjectPath || tree.length === 0) return;

    // Helper to find a node by path
    function findNode(list: FileEntry[], target: string): FileEntry | null {
        for (const node of list) {
            if (node.path === target) return node;
            if (node.is_dir && node.children) {
                const found = findNode(node.children, target);
                if (found) return found;
            }
        }
        return null;
    }

    // If the target is a file, refresh its parent directory instead
    let dirPath = path;
    const targetNode = findNode(tree, path);
    if (targetNode && !targetNode.is_dir) {
        dirPath = path.split('/').slice(0, -1).join('/') || currentProjectPath;
    }

    // For project root, ensure we reload children directly under root
    const effectiveDir = dirPath === currentProjectPath ? currentProjectPath : dirPath;
    try {
        const newChildren: FileEntry[] = await invoke('list_files', { dirPath: effectiveDir });
        for (const f of newChildren) {
            f.level = effectiveDir === currentProjectPath ? 1 : (targetNode?.level ?? 0) + 1;
            f.parent_dir = effectiveDir|| '';
            if (f.is_dir) {
                f.expanded = f.expanded ?? false;
                f.children = f.children ?? [];
            }
        }

        fileStore.update((state) => {
            const clone = (n: FileEntry): FileEntry => ({
                ...n,
                children: n.children ? n.children.map(clone) : n.children,
            });
            const root = state.files[0] ? clone(state.files[0]) : makeRoot(currentProjectPath!);

            function replaceChildren(node: FileEntry, atPath: string, children: FileEntry[]) {
                if (node.path === atPath) {
                    node.children = children;
                    return true;
                }
                if (node.children) {
                    for (const child of node.children) {
                        if (child.is_dir && replaceChildren(child, atPath, children)) return true;
                    }
                }
                return false;
            }

            if (effectiveDir === currentProjectPath) {
                root.children = newChildren;
            } else {
                replaceChildren(root, effectiveDir, newChildren);
            }

            return {
                ...state,
                files: [root],
            };
        });
    } catch (e) {
        console.error('Failed to refresh path in store:', e);
    }
}

// Function to select a file
export function selectFile(file: FileEntry) {
    fileStore.update((state) => ({
        ...state,
        selectedFile: file,
    }));
}