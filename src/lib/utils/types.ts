export interface FileEntry {
    path: string;
    name: string;
    is_dir: boolean;
    expanded?: boolean;
    children?: FileEntry[];
    level?: number;
    parent_dir?: string;
    temp?: boolean;
    isEditing?: boolean;
}

export type ProjectType = 'NPM' | 'Rust' | 'blank';

export interface RunConfig {
    id: string;
    name: string;
    command: string;
    cwd: string;
}

export interface ProjectInfo {
    name: string;
    path: string;
    project_type: ProjectType;
    run_configs: RunConfig[];
}

export interface ProjectEntry extends ProjectInfo {
    selected_run_config_id?: string | null;
}