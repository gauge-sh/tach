import os
from dataclasses import dataclass, field
from typing import Optional

from modguard import filesystem as fs
from modguard.core import ModuleTrie, ModuleNode, ProjectConfig
from modguard.parsing.modules import build_module_trie
from modguard.parsing.imports import get_project_imports


@dataclass
class ErrorInfo:
    location: str = ""
    import_mod_path: str = ""
    source_scope: str = ""
    allowed_scopes: list[str] = field(default_factory=list)
    exception_message: str = ""

    @property
    def message(self) -> str:
        if self.exception_message:
            return self.exception_message
        if not all(
            (
                self.location,
                self.import_mod_path,
                self.source_scope,
                self.allowed_scopes,
            )
        ):
            return f"Unexpected error: ({[self.location, self.import_mod_path, self.source_scope, self.allowed_scopes]})"
        return f"Import '{self.import_mod_path}' in {self.location} is blocked. Scope '{self.source_scope}' can only depend on scopes '{self.allowed_scopes}'."


@dataclass
class CheckResult:
    ok: bool
    error_info: Optional[ErrorInfo] = None

    @classmethod
    def success(cls) -> "CheckResult":
        return cls(ok=True)

    @classmethod
    def fail(cls, error_info: ErrorInfo) -> "CheckResult":
        return cls(ok=False, error_info=error_info)


def check_import(
    project_config: ProjectConfig,
    module_trie: ModuleTrie,
    import_mod_path: str,
    file_nearest_module: ModuleNode,
    file_mod_path: str,
) -> CheckResult:
    import_nearest_module = module_trie.find_nearest(import_mod_path)
    if import_nearest_module is None:
        # This shouldn't happen since we intend to filter out any external imports,
        # but we should allow external imports if they have made it here.
        return CheckResult.success()

    # The import must be explicitly allowed based on the scopes and top-level config
    file_scopes = file_nearest_module.config.scopes
    import_scopes = import_nearest_module.config.scopes

    for scope in file_scopes:
        dependency_tags = (
            project_config.dependency_rules[scope].depends_on
            if scope in project_config.dependency_rules
            else []
        )
        if "*" in dependency_tags:
            continue
        if any((scope in dependency_tags for scope in import_scopes)):
            continue
        # This means the import has scopes which the file cannot depend on
        return CheckResult.fail(
            error_info=ErrorInfo(
                location=file_mod_path,
                import_mod_path=import_mod_path,
                source_scope=scope,
                allowed_scopes=dependency_tags,
            )
        )

    return CheckResult.success()


def check(
    root: str, project_config: ProjectConfig, exclude_paths: Optional[list[str]] = None
) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [
            ErrorInfo(exception_message=f"The path {root} is not a valid directory.")
        ]

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = fs.canonical(root)
    exclude_paths = list(map(fs.canonical, exclude_paths)) if exclude_paths else None

    module_trie = build_module_trie(root, exclude_paths=exclude_paths)

    errors: list[ErrorInfo] = []
    for file_path in fs.walk_pyfiles(root, exclude_paths=exclude_paths):
        mod_path = fs.file_to_module_path(file_path)
        nearest_module = module_trie.find_nearest(mod_path)
        if nearest_module is None:
            continue
        import_mod_paths = get_project_imports(root, file_path)
        for import_mod_path in import_mod_paths:
            check_result = check_import(
                project_config=project_config,
                module_trie=module_trie,
                import_mod_path=import_mod_path,
                file_nearest_module=nearest_module,
                file_mod_path=mod_path,
            )
            if check_result.ok:
                # This import is OK
                continue

            errors.append(check_result.error_info)

    return errors
