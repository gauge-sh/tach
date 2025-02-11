use std::path::PathBuf;

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::Mod;
use thiserror::Error;

use crate::config::plugins::django::DjangoConfig;
use crate::dependencies::SourceCodeReference;
use crate::filesystem;
use crate::python::error::ParsingError;
use crate::python::parsing::parse_python_source;

#[derive(Error, Debug)]
pub enum FKeyError {
    #[error("Failed to parse Django foreign keys: {0}")]
    Parsing(#[from] ParsingError),
    #[error("Failed to read Django settings file: {0}")]
    Filesystem(#[from] filesystem::FileSystemError),
    #[error("Could not find Django settings file: {0}")]
    SettingsFileNotFound(String),
}

pub type Result<T> = std::result::Result<T, FKeyError>;

struct FKeyVisitor {
    pub fkeys: Vec<SourceCodeReference>,
}

impl FKeyVisitor {
    fn new() -> Self {
        FKeyVisitor { fkeys: vec![] }
    }
}

impl Visitor<'_> for FKeyVisitor {
    fn visit_expr(&mut self, expr: &ruff_python_ast::Expr) {
        if let ruff_python_ast::Expr::Call(call_expr) = expr {
            let is_foreign_key = match &*call_expr.func {
                // Match direct name (ForeignKey)
                ruff_python_ast::Expr::Name(name) => name.id() == "ForeignKey",
                // Match attribute access (models.ForeignKey, django.db.models.ForeignKey, etc.)
                ruff_python_ast::Expr::Attribute(attr) => attr.attr.as_str() == "ForeignKey",
                _ => false,
            };

            if !is_foreign_key {
                return;
            }

            let target_model = if !call_expr.arguments.args.is_empty() {
                // First positional argument
                if let ruff_python_ast::Expr::StringLiteral(s) = &call_expr.arguments.args[0] {
                    Some((s.value.to_string(), s.range.start()))
                } else {
                    None
                }
            } else {
                // Look for "to" keyword argument
                call_expr.arguments.keywords.iter().find_map(|kw| {
                    if kw.arg.as_deref() == Some("to") {
                        if let ruff_python_ast::Expr::StringLiteral(s) = &kw.value {
                            Some((s.value.to_string(), s.range.start()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            };

            if let Some((model_path, string_start)) = target_model {
                self.fkeys
                    .push(SourceCodeReference::new(model_path, string_start));
            }
        }
    }
}

struct InstalledAppVisitor {
    pub installed_apps: Vec<String>,
}

impl InstalledAppVisitor {
    fn new() -> Self {
        InstalledAppVisitor {
            installed_apps: vec![],
        }
    }
}

impl Visitor<'_> for InstalledAppVisitor {
    fn visit_stmt(&mut self, stmt: &ruff_python_ast::Stmt) {
        if let ruff_python_ast::Stmt::Assign(ref assign) = stmt {
            if assign.targets.len() == 1 {
                let target = &assign.targets[0];
                if let ruff_python_ast::Expr::Name(name) = target {
                    if name.id() == "INSTALLED_APPS" {
                        if let ruff_python_ast::Expr::List(list) = assign.value.as_ref() {
                            for item in &list.elts {
                                if let ruff_python_ast::Expr::StringLiteral(string) = item {
                                    self.installed_apps.push(string.value.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn filter_installed_apps(source_roots: &[PathBuf], installed_apps: Vec<String>) -> Vec<String> {
    installed_apps
        .into_iter()
        .filter(|app| filesystem::module_to_file_path(source_roots, app, false).is_some())
        .collect()
}

pub fn get_known_apps(
    source_roots: &[PathBuf],
    django_config: &DjangoConfig,
) -> Result<Vec<String>> {
    let settings_module_path = &django_config.settings_module;
    let settings_module =
        filesystem::module_to_file_path(source_roots, settings_module_path, false);
    if let Some(settings_module) = settings_module {
        let settings_file = filesystem::read_file_content(settings_module.file_path)?;
        let settings_ast = parse_python_source(&settings_file)?;
        let mut visitor = InstalledAppVisitor::new();

        match settings_ast {
            Mod::Module(ref module) => {
                visitor.visit_body(&module.body);
            }
            Mod::Expression(_) => return Err(FKeyError::Parsing(ParsingError::InvalidSyntax)),
        };

        Ok(filter_installed_apps(source_roots, visitor.installed_apps))
    } else {
        Err(FKeyError::SettingsFileNotFound(
            settings_module_path.to_string(),
        ))
    }
}

pub fn get_foreign_key_references(file_ast: &Mod) -> impl IntoIterator<Item = SourceCodeReference> {
    let mut visitor = FKeyVisitor::new();

    if let Mod::Module(module) = file_ast {
        visitor.visit_body(&module.body);
    }

    visitor.fkeys
}
