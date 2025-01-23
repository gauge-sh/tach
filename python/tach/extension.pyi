from pathlib import Path
from typing import Literal

def get_project_imports(
    source_roots: list[str],
    file_path: str,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> list[tuple[str, int]]: ...
def get_external_imports(
    source_roots: list[str],
    file_path: str,
    ignore_type_checking_imports: bool,
) -> list[tuple[str, int]]: ...
def get_normalized_imports(
    source_roots: list[str],
    file_path: str,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> list[tuple[str, int]]: ...
def set_excluded_paths(
    project_root: str, exclude_paths: list[str], use_regex_matching: bool
) -> None: ...
def check_external_dependencies(
    project_root: str,
    project_config: ProjectConfig,
    module_mappings: dict[str, list[str]],
    stdlib_modules: list[str],
) -> ExternalCheckDiagnostics: ...
def create_dependency_report(
    project_root: str,
    project_config: ProjectConfig,
    path: str,
    include_dependency_modules: list[str] | None,
    include_usage_modules: list[str] | None,
    skip_dependencies: bool,
    skip_usages: bool,
    raw: bool,
) -> str: ...
def create_computation_cache_key(
    project_root: str,
    source_roots: list[str],
    action: str,
    py_interpreter_version: str,
    file_dependencies: list[str],
    env_dependencies: list[str],
    backend: str,
) -> str: ...
def check_computation_cache(
    project_root: str, cache_key: str
) -> tuple[list[tuple[int, str]], int] | None: ...
def update_computation_cache(
    project_root: str, cache_key: str, value: tuple[list[tuple[int, str]], int]
) -> None: ...
def parse_project_config(filepath: Path) -> tuple[ProjectConfig, bool]: ...
def dump_project_config_to_toml(project_config: ProjectConfig) -> str: ...
def check(
    project_root: Path,
    project_config: ProjectConfig,
    dependencies: bool,
    interfaces: bool,
    exclude_paths: list[str],
) -> CheckDiagnostics: ...
def detect_unused_dependencies(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
) -> list[UnusedDependencies]: ...
def sync_project(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
    add: bool = False,
) -> None: ...
def run_server(project_root: Path, project_config: ProjectConfig) -> None: ...
def serialize_modules_json(modules: list[ModuleConfig]) -> str: ...

class ErrorInfo:
    def is_dependency_error(self) -> bool: ...
    def is_interface_error(self) -> bool: ...
    def to_pystring(self) -> str: ...
    def is_deprecated(self) -> bool: ...

class BoundaryError:
    file_path: Path
    line_number: int
    import_mod_path: str
    error_info: ErrorInfo

class CheckDiagnostics:
    errors: list[BoundaryError]
    deprecated_warnings: list[BoundaryError]
    warnings: list[str]

    def serialize_json(self, pretty_print: bool = False) -> str: ...

class ExternalCheckDiagnostics:
    undeclared_dependencies: dict[str, list[str]]
    unused_dependencies: dict[str, list[str]]
    errors: list[str]
    warnings: list[str]

    def __new__(
        cls,
        undeclared_dependencies: dict[str, list[str]],
        unused_dependencies: dict[str, list[str]],
        errors: list[str],
        warnings: list[str],
    ) -> ExternalCheckDiagnostics: ...

class DependencyConfig:
    path: str
    deprecated: bool

class ModuleConfig:
    path: str
    depends_on: list[DependencyConfig] | None
    visibility: list[str]
    strict: bool
    unchecked: bool

    def __new__(cls, path: str, strict: bool) -> ModuleConfig: ...
    def mod_path(self) -> str: ...

InterfaceDataTypes = Literal["all", "primitive"]

class InterfaceConfig:
    expose: list[str]
    # 'from' in tach.toml
    from_modules: list[str]
    data_types: InterfaceDataTypes

CacheBackend = Literal["disk"]

class CacheConfig:
    backend: CacheBackend
    file_dependencies: list[str]
    env_dependencies: list[str]

class ExternalDependencyConfig:
    exclude: list[str]
    rename: list[str]

class UnusedDependencies:
    path: str
    dependencies: list[DependencyConfig]

RuleSetting = Literal["error", "warn", "off"]

class RulesConfig:
    unused_ignore_directives: RuleSetting
    require_ignore_directive_reasons: RuleSetting

class ProjectConfig:
    cache: CacheConfig
    external: ExternalDependencyConfig
    exclude: list[str]
    source_roots: list[str]
    exact: bool
    disable_logging: bool
    ignore_type_checking_imports: bool
    include_string_imports: bool
    forbid_circular_dependencies: bool
    use_regex_matching: bool
    rules: RulesConfig

    def __new__(cls) -> ProjectConfig: ...
    def serialize_json(self) -> str: ...
    def set_location(self, location: Path) -> None: ...
    def has_no_modules(self) -> bool: ...
    def has_no_dependencies(self) -> bool: ...
    def module_paths(self) -> list[str]: ...
    def utility_paths(self) -> list[str]: ...
    def create_module(self, path: str) -> None: ...
    def delete_module(self, path: str) -> None: ...
    def mark_module_as_utility(self, path: str) -> None: ...
    def unmark_module_as_utility(self, path: str) -> None: ...
    def add_dependency(self, path: str, dependency: str) -> None: ...
    def remove_dependency(self, path: str, dependency: str) -> None: ...
    def add_source_root(self, path: Path) -> None: ...
    def remove_source_root(self, path: Path) -> None: ...
    def save_edits(self) -> None: ...
    def all_modules(self) -> list[ModuleConfig]: ...
    def all_interfaces(self) -> list[InterfaceConfig]: ...
    def filtered_modules(self, included_paths: list[Path]) -> list[ModuleConfig]: ...

class TachPytestPluginHandler:
    removed_test_paths: set[str]
    all_affected_modules: set[str]
    num_removed_items: int
    tests_ran_to_completion: bool
    def __new__(
        cls,
        project_root: Path,
        project_config: ProjectConfig,
        changed_files: list[Path],
        all_affected_modules: set[Path],
    ) -> TachPytestPluginHandler: ...
    def remove_test_path(self, path: Path) -> None: ...
    def should_remove_items(self, file_path: Path) -> bool: ...
