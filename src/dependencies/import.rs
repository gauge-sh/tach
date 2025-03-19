use std::collections::HashMap;

use ruff_text_size::TextSize;

use crate::{
    external::parsing::normalize_package_name,
    resolvers::{PackageResolution, PackageResolver},
};

/// An import with a normalized module path
#[derive(Debug, Clone)]
pub struct NormalizedImport {
    pub module_path: String,        // Global module path
    pub alias_path: Option<String>, // (for relative imports) alias path
    pub import_offset: TextSize,    // Source location of the import statement
    pub alias_offset: TextSize,     // Source location of the alias
    pub is_absolute: bool,          // Whether the import is absolute
    pub is_global_scope: bool,      // Whether the import is at the global scope
}

impl NormalizedImport {
    pub fn top_level_module_name(&self) -> &str {
        self.module_path
            .split('.')
            .next()
            .expect("Normalized import module path is empty")
    }
}

#[derive(Debug, Clone)]
pub struct LocatedImport {
    pub import: NormalizedImport,
    pub import_line_number: usize,
    pub alias_line_number: usize,
}

impl LocatedImport {
    pub fn new(
        import_line_number: usize,
        alias_line_number: usize,
        import: NormalizedImport,
    ) -> Self {
        Self {
            import,
            import_line_number,
            alias_line_number,
        }
    }

    pub fn module_path(&self) -> &str {
        &self.import.module_path
    }

    pub fn alias_path(&self) -> Option<&str> {
        self.import.alias_path.as_deref()
    }

    pub fn import_line_number(&self) -> usize {
        self.import_line_number
    }

    pub fn alias_line_number(&self) -> usize {
        self.alias_line_number
    }

    pub fn is_absolute(&self) -> bool {
        self.import.is_absolute
    }
}

#[derive(Debug)]
pub struct ExternalImportWithDistributionNames<'a> {
    pub distribution_names: Vec<String>,
    pub import: &'a NormalizedImport,
}

impl ExternalImportWithDistributionNames<'_> {
    pub fn top_level_module_name(&self) -> &str {
        self.import.top_level_module_name()
    }

    pub fn alias_offset(&self) -> TextSize {
        self.import.alias_offset
    }

    pub fn import_offset(&self) -> TextSize {
        self.import.import_offset
    }

    pub fn is_global_scope(&self) -> bool {
        self.import.is_global_scope
    }

    pub fn distribution_names(&self) -> &Vec<String> {
        &self.distribution_names
    }
}

pub fn with_distribution_names<'a, I>(
    imports: I,
    package_resolver: &PackageResolver,
    module_mappings: &HashMap<String, Vec<String>>,
) -> Vec<ExternalImportWithDistributionNames<'a>>
where
    I: Iterator<Item = &'a NormalizedImport>,
{
    imports
        .map(|import| {
            let top_level_module_name = import.top_level_module_name().to_string();
            let default_distribution_names =
                match package_resolver.resolve_module_path(&import.module_path) {
                    PackageResolution::Found { package, .. } => {
                        vec![package
                            .name
                            .as_ref()
                            .map(|name| normalize_package_name(name))
                            .unwrap_or_else(|| top_level_module_name.clone())]
                    }
                    PackageResolution::NotFound | PackageResolution::Excluded => {
                        vec![top_level_module_name.clone()]
                    }
                };
            let distribution_names: Vec<String> = module_mappings
                .get(&top_level_module_name)
                .map(|dist_names| {
                    dist_names
                        .iter()
                        .map(|dist_name| normalize_package_name(dist_name))
                        .collect()
                })
                .unwrap_or(default_distribution_names);

            ExternalImportWithDistributionNames {
                distribution_names,
                import,
            }
        })
        .collect()
}
