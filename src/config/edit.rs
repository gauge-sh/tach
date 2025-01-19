#[derive(Debug, Clone, PartialEq)]
pub enum ConfigEdit {
    CreateModule { path: String },
    DeleteModule { path: String },
    MarkModuleAsUtility { path: String },
    UnmarkModuleAsUtility { path: String },
    AddDependency { path: String, dependency: String },
    RemoveDependency { path: String, dependency: String },
}
