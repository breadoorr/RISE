export interface FileEntry {
    path: string;
    name: string;
    is_dir: boolean;
    expanded?: boolean;
    children?: FileEntry[];
    level?: number;
    parent_dir?: string;
}