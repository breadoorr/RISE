import { invoke } from "@tauri-apps/api/core";
import type { FileEntry } from "./types";
import { basename, dirname } from "@tauri-apps/api/path";

// Pure helper to load directory contents and normalize fields without holding module state
export async function loadFiles(path: string, level: number = 0): Promise<FileEntry[]> {
  try {
    const dirFiles = await invoke("list_files", { dirPath: path }) as FileEntry[];

    for (const file of dirFiles) {
      file.level = level;
      const parent = await dirname(path);
      file.parent_dir = await basename(parent) || "";
      if (file.is_dir) {
        file.expanded = false;
        file.children = [];
      }
    }

    return dirFiles as FileEntry[];
  } catch (error) {
    console.error("Error listing files:", error);
    return [] as FileEntry[];
  }
}

// Pure helper that flattens a hierarchical file list into a single array
export function updateAllFiles(files: FileEntry[]): FileEntry[] {
  const all: FileEntry[] = [];
  function flattenFiles(list: FileEntry[]) {
    for (const file of list) {
      all.push(file);
      if (file.is_dir && file.expanded && file.children) {
        flattenFiles(file.children);
      }
    }
  }
  flattenFiles(files);
  return all as FileEntry[];
}
