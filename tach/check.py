import os
import re
from dataclasses import dataclass, field
from typing import Optional

from tach import filesystem as fs
from tach.core import PackageTrie, PackageNode, ProjectConfig
from tach.parsing import build_package_trie, get_project_imports


@dataclass
class ErrorInfo:
    location: str = ""
    import_mod_path: str = ""
    source_tag: str = ""
    invalid_tags: list[str] = field(default_factory=list)
    allowed_tags: list[str] = field(default_factory=list)
    exception_message: str = ""

    @property
    def is_tag_error(self) -> bool:
        return all(
            (self.location, self.import_mod_path, self.source_tag, self.invalid_tags)
        )

    @property
    def message(self) -> str:
        if self.exception_message:
            return self.exception_message
        if not self.is_tag_error:
            return f"Unexpected error: ({[self.location, self.import_mod_path, self.source_tag, self.allowed_tags]})"
        if not self.allowed_tags:
            return (
                f"Import '{self.import_mod_path}' with tags '{self.invalid_tags}' "
                f"in {self.location} is blocked. "
                f"Tag '{self.source_tag}' has no allowed dependency tags."
            )
        return (
            f"Import '{self.import_mod_path}' with tags '{self.invalid_tags}' "
            f"in {self.location} is blocked. "
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


def is_top_level_package_import(mod_path: str, package: PackageNode) -> bool:
    return mod_path == package.full_path


def import_matches_interface_members(mod_path: str, package: PackageNode) -> bool:
    mod_path_segments = mod_path.rsplit(".", 1)
    if len(mod_path_segments) == 1:
        return mod_path_segments[0] == package.full_path
    else:
        mod_pkg_path, mod_member_name = mod_path_segments
        return (
            mod_pkg_path == package.full_path
            and mod_member_name in package.interface_members
        )


def check_import(
    project_config: ProjectConfig,
    package_trie: PackageTrie,
    import_mod_path: str,
    file_mod_path: str,
    file_nearest_package: Optional[PackageNode] = None,
) -> CheckResult:
    import_nearest_package = package_trie.find_nearest(import_mod_path)
    if import_nearest_package is None:
        # This shouldn't happen since we intend to filter out any external imports,
        # but we should allow external imports if they have made it here.
        return CheckResult.success()

    # Lookup file_mod_path if package not given
    if file_nearest_package is None:
        file_nearest_package = package_trie.find_nearest(file_mod_path)
    # If package not found, we should fail since the implication is that
    # an external package is importing directly from our project
    if file_nearest_package is None:
        return CheckResult.fail(
            error_info=ErrorInfo(
                exception_message=f"Package containing '{file_mod_path}' not found in project."
            )
        )

    # Imports within the same package are always allowed
    if import_nearest_package == file_nearest_package:
        return CheckResult.success()

    import_package_config = import_nearest_package.config
    if import_package_config and import_package_config.strict:
        if not is_top_level_package_import(
            import_mod_path, import_nearest_package
        ) and not import_matches_interface_members(
            import_mod_path, import_nearest_package
        ):
            # In strict mode, import must be of the package itself or one of the
            # interface members (defined in __all__)
            return CheckResult.fail(
                error_info=ErrorInfo(
                    location=file_mod_path,
                    exception_message=(
                        f"Package '{import_nearest_package.full_path}' is in strict mode. "
                        "Only imports from the root of this package are allowed. "
                        f"The import '{import_mod_path}' (in '{file_mod_path}') "
                        f"is not included in __all__."
                    ),
                )
            )

    # The import must be explicitly allowed based on the tags and top-level config
    if not file_nearest_package.config or not import_nearest_package.config:
        return CheckResult.fail(
            error_info=ErrorInfo(
                exception_message="Could not find config for packages."
            )
        )
    file_tags = file_nearest_package.config.tags
    import_tags = import_nearest_package.config.tags

    for file_tag in file_tags:
        dependency_tags = project_config.dependencies_for_tag(file_tag)
        if any(
            any(
                re.match(dependency_tag, import_tag)
                for dependency_tag in dependency_tags
            )
            for import_tag in import_tags
        ):
            # The import has at least one tag which matches at least one expected dependency
            continue
        # This means the import has no tags which the file can depend on
        return CheckResult.fail(
            error_info=ErrorInfo(
                location=file_mod_path,
                import_mod_path=import_mod_path,
                source_tag=file_tag,
                invalid_tags=import_tags,
                allowed_tags=dependency_tags,
            )
        )

    return CheckResult.success()


def check(
    root: str,
    project_config: ProjectConfig,
    exclude_paths: Optional[list[str]] = None,
    exclude_hidden_paths: Optional[bool] = True,
) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [
            ErrorInfo(exception_message=f"The path {root} is not a valid directory.")
        ]

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = fs.canonical(root)
    exclude_paths = list(map(fs.canonical, exclude_paths)) if exclude_paths else None

    package_trie = build_package_trie(
        root, exclude_paths=exclude_paths, exclude_hidden_paths=exclude_hidden_paths
    )

    errors: list[ErrorInfo] = []
    for file_path in fs.walk_pyfiles(
        root, exclude_paths=exclude_paths, exclude_hidden_paths=exclude_hidden_paths
    ):
        mod_path = fs.file_to_module_path(file_path)
        nearest_package = package_trie.find_nearest(mod_path)
        if nearest_package is None:
            continue
        import_mod_paths = get_project_imports(root, file_path)
        # This should only give us imports from within our project
        # (excluding stdlib, builtins, and 3rd party packages)
        for import_mod_path in import_mod_paths:
            check_result = check_import(
                project_config=project_config,
                package_trie=package_trie,
                import_mod_path=import_mod_path,
                file_nearest_package=nearest_package,
                file_mod_path=mod_path,
            )
            if check_result.ok or check_result.error_info is None:
                continue

            errors.append(check_result.error_info)

    return errors
