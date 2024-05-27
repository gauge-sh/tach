from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Optional

from tach import errors
from tach import filesystem as fs
from tach.parsing import build_package_trie, get_project_imports

if TYPE_CHECKING:
    from tach.core import PackageNode, PackageTrie, ProjectConfig


@dataclass
class ErrorInfo:
    source_tags: list[str] = field(default_factory=list)
    invalid_tags: list[str] = field(default_factory=list)
    allowed_tags: list[str] = field(default_factory=list)
    exception_message: str = ""

    @property
    def is_tag_error(self) -> bool:
        return all((self.source_tags, self.invalid_tags))


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
) -> Optional[ErrorInfo]:
    import_nearest_package = package_trie.find_nearest(import_mod_path)
    if import_nearest_package is None:
        # This shouldn't happen since we intend to filter out any external imports,
        # but we should allow external imports if they have made it here.
        return None

    # Lookup file_mod_path if package not given
    if file_nearest_package is None:
        file_nearest_package = package_trie.find_nearest(file_mod_path)
    # If package not found, we should fail since the implication is that
    # an external package is importing directly from our project
    if file_nearest_package is None:
        return ErrorInfo(
            exception_message=f"Package containing '{file_mod_path}' not found in project.",
        )

    # Imports within the same package are always allowed
    if import_nearest_package == file_nearest_package:
        return None

    import_package_config = import_nearest_package.config
    if import_package_config and import_package_config.strict:
        if not is_top_level_package_import(
            import_mod_path, import_nearest_package
        ) and not import_matches_interface_members(
            import_mod_path, import_nearest_package
        ):
            # In strict mode, import must be of the package itself or one of the
            # interface members (defined in __all__)
            return ErrorInfo(
                exception_message=(
                    f"Package '{import_nearest_package.full_path}' is in strict mode. "
                    "Only imports from the root of this package are allowed. "
                    f"The import '{import_mod_path}' (in '{file_mod_path}') "
                    f"is not included in __all__."
                ),
            )

    # The import must be explicitly allowed based on the tags and top-level config
    if not file_nearest_package.config or not import_nearest_package.config:
        return ErrorInfo(
            exception_message="Could not find package configuration.",
        )
    file_tags = file_nearest_package.config.tags
    import_tags = import_nearest_package.config.tags

    for file_tag in file_tags:
        dependency_tags = project_config.dependencies_for_tag(file_tag)
        if any(
            any(dependency_tag == import_tag for dependency_tag in dependency_tags)
            for import_tag in import_tags
        ):
            # The import has at least one tag which matches at least one expected dependency
            continue
        # This means the import has no tags which the file can depend on
        return ErrorInfo(
            source_tags=file_tags,
            invalid_tags=import_tags,
            allowed_tags=dependency_tags,
        )

    return None


@dataclass
class BoundaryError:
    file_path: str
    line_number: int
    import_mod_path: str
    error_info: ErrorInfo


def check(
    root: str,
    project_config: ProjectConfig,
    exclude_paths: Optional[list[str]] = None,
) -> list[BoundaryError]:
    if not os.path.isdir(root):
        raise errors.TachSetupError(f"The path {root} is not a valid directory.")

    cwd = fs.get_cwd()
    try:
        fs.chdir(root)

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        package_trie = build_package_trie(
            ".",
            exclude_paths=exclude_paths,
        )

        boundary_errors: list[BoundaryError] = []
        for file_path in fs.walk_pyfiles(
            ".",
            exclude_paths=exclude_paths,
        ):
            mod_path = fs.file_to_module_path(file_path)
            nearest_package = package_trie.find_nearest(mod_path)
            if nearest_package is None:
                continue

            # This should only give us imports from within our project
            # (excluding stdlib, builtins, and 3rd party packages)
            project_imports = get_project_imports(
                ".",
                file_path,
                ignore_type_checking_imports=project_config.ignore_type_checking_imports,
            )
            for project_import in project_imports:
                check_error = check_import(
                    project_config=project_config,
                    package_trie=package_trie,
                    import_mod_path=project_import.mod_path,
                    file_nearest_package=nearest_package,
                    file_mod_path=mod_path,
                )
                if check_error is None:
                    continue

                boundary_errors.append(
                    BoundaryError(
                        file_path=file_path,
                        import_mod_path=project_import.mod_path,
                        line_number=project_import.line_number,
                        error_info=check_error,
                    )
                )

        return boundary_errors
    finally:
        fs.chdir(cwd)
