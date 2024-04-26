import os
import re
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
    source_tag: str = ""
    allowed_tags: list[str] = field(default_factory=list)
    exception_message: str = ""

    @property
    def message(self) -> str:
        if self.exception_message:
            return self.exception_message
        if not all(
            (
                self.location,
                self.import_mod_path,
                self.source_tag,
            )
        ):
            return f"Unexpected error: ({[self.location, self.import_mod_path, self.source_tag, self.allowed_tags]})"
        if not self.allowed_tags:
            return (
                f"Import '{self.import_mod_path}' in {self.location} is blocked. "
                f"Tag '{self.source_tag}' can only depend on tags '{self.allowed_tags}'."
            )
        return (
            f"Import '{self.import_mod_path}' in {self.location} is blocked. "
            f"Tag '{self.source_tag}' can only depend on tags '{self.allowed_tags}'."
        )


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
    file_mod_path: str,
    file_nearest_module: Optional[ModuleNode] = None,
) -> CheckResult:
    import_nearest_module = module_trie.find_nearest(import_mod_path)
    if import_nearest_module is None:
        # This shouldn't happen since we intend to filter out any external imports,
        # but we should allow external imports if they have made it here.
        return CheckResult.success()

    # Lookup file_mod_path if module not given
    if file_nearest_module is None:
        file_nearest_module = module_trie.find_nearest(file_mod_path)
    # If module not found, we should fail since the implication is that
    # an external module is importing directly from our project
    if file_nearest_module is None:
        return CheckResult.fail(
            error_info=ErrorInfo(
                exception_message=f"Module '{file_mod_path}' not found in project."
            )
        )

    # Imports within the same module are always allowed
    if import_nearest_module == file_nearest_module:
        return CheckResult.success()

    import_module_config = import_nearest_module.config
    if (
        import_module_config
        and import_module_config.strict
        and import_mod_path != import_nearest_module.full_path
    ):
        # Must import from module's full path exactly in strict mode
        return CheckResult.fail(
            error_info=ErrorInfo(
                exception_message=(
                    f"Module '{import_nearest_module.full_path}' is in strict mode. "
                    "Only imports from the root of this module are allowed. "
                    f"The import '{import_mod_path}' does not match the root ('{import_nearest_module.full_path}')."
                )
            )
        )

    # The import must be explicitly allowed based on the tags and top-level config
    if not file_nearest_module.config or not import_nearest_module.config:
        return CheckResult.fail(
            error_info=ErrorInfo(exception_message="Could not find config for modules.")
        )
    file_tags = file_nearest_module.config.tags
    import_tags = import_nearest_module.config.tags

    for file_tag in file_tags:
        dependency_tags = (
            project_config.constraints[file_tag].depends_on
            if file_tag in project_config.constraints
            else []
        )
        if any(
            any(
                re.match(dependency_tag, import_tag)
                for dependency_tag in dependency_tags
            )
            for import_tag in import_tags
        ):
            # The import has at least one tag which matches at least one expected dependency
            continue
        # This means the import has scopes which the file cannot depend on
        return CheckResult.fail(
            error_info=ErrorInfo(
                location=file_mod_path,
                import_mod_path=import_mod_path,
                source_tag=file_tag,
                allowed_tags=dependency_tags,
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
        # This should only give us imports from within our project
        # (excluding stdlib, builtins, and 3rd party packages)
        for import_mod_path in import_mod_paths:
            check_result = check_import(
                project_config=project_config,
                module_trie=module_trie,
                import_mod_path=import_mod_path,
                file_nearest_module=nearest_module,
                file_mod_path=mod_path,
            )
            if check_result.ok or check_result.error_info is None:
                continue

            errors.append(check_result.error_info)

    return errors
