
pub enum ProjectType {
    Rust,
    Node,
    Other
}
pub struct Project {
    pub name: String,
    pub path: String,
    pub project_type: ProjectType
}