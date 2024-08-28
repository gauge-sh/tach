from __future__ import annotations

import fnmatch
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.extension import get_project_imports, set_excluded_paths
from tach.parsing import build_module_tree

if TYPE_CHECKING:
    from tach.core import (
        Dependency,  # noqa: TCH004
        ModuleNode,
        ModuleTree,
        ProjectConfig,
    )


@dataclass
class ErrorInfo:
    source_module: str = ""
    invalid_module: str = ""
    allowed_dependencies: list[Dependency] = field(default_factory=list)
    deprecated_dependencies: list[Dependency] = field(default_factory=list)
    exception_message: str = ""

    @property
    def is_dependency_error(self) -> bool:
        return all((self.source_module, self.invalid_module))

    @property
    def is_deprecated(self) -> bool:
        return self.is_dependency_error and self.invalid_module in [
            dep.path for dep in self.deprecated_dependencies
        ]


def is_top_level_module_import(mod_path: str, module: ModuleNode) -> bool:
    return mod_path == module.full_path


def import_matches_interface_members(mod_path: str, module: ModuleNode) -> bool:
    mod_path_segments = mod_path.rsplit(".", 1)
    if len(mod_path_segments) == 1:
        return mod_path_segments[0] == module.full_path
    else:
        mod_pkg_path, mod_member_name = mod_path_segments
        return (
            mod_pkg_path == module.full_path
            and mod_member_name in module.interface_members
        )


def check_import(
    module_tree: ModuleTree,
    import_mod_path: str,
    file_mod_path: str,
    file_nearest_module: ModuleNode | None = None,
) -> ErrorInfo | None:
    import_nearest_module = module_tree.find_nearest(import_mod_path)
    if import_nearest_module is None:
        # This shouldn't happen since we intend to filter out any external imports,
        # but we should allow external imports if they have made it here.
        return None

    # Lookup file_mod_path if module not given
    if file_nearest_module is None:
        file_nearest_module = module_tree.find_nearest(file_mod_path)
    # If module not found, we should fail since the implication is that
    # an external module is importing directly from our project
    if file_nearest_module is None:
        return ErrorInfo(
            exception_message=f"Module containing '{file_mod_path}' not found in project.",
        )

    # Imports within the same module are always allowed
    if import_nearest_module == file_nearest_module:
        return None

    import_module_config = import_nearest_module.config
    if import_module_config and import_module_config.strict:
        if not is_top_level_module_import(
            import_mod_path, import_nearest_module
        ) and not import_matches_interface_members(
            import_mod_path, import_nearest_module
        ):
            # In strict mode, import must be of the module itself or one of the
            # interface members (defined in __all__)
            return ErrorInfo(
                exception_message=(
                    f"Module '{import_nearest_module.full_path}' is in strict mode. "
                    "Only imports from the public interface of this module are allowed. "
                    f"The import '{import_mod_path}' (in module '{file_nearest_module.full_path}') "
                    f"is not included in __all__."
                ),
            )

    if not file_nearest_module.config or not import_nearest_module.config:
        return ErrorInfo(
            exception_message="Could not find module configuration.",
        )

    file_nearest_module_path = file_nearest_module.config.path
    import_nearest_module_path = import_nearest_module.config.path

    # The import must be explicitly allowed
    dependencies = file_nearest_module.config.depends_on

    allowed_dependencies = [dep for dep in dependencies if not dep.deprecated]
    deprecated_dependencies = [dep for dep in dependencies if dep.deprecated]
    if any(dep.path == import_nearest_module_path for dep in allowed_dependencies):
        # The import matches at least one expected dependency
        return None
    if any(dep.path == import_nearest_module_path for dep in deprecated_dependencies):
        # Dependency exists but is deprecated
        return ErrorInfo(
            source_module=file_nearest_module_path,
            invalid_module=import_nearest_module_path,
            allowed_dependencies=allowed_dependencies,
            deprecated_dependencies=deprecated_dependencies,
        )
    # This means the import is not declared as a dependency of the file
    return ErrorInfo(
        source_module=file_nearest_module_path,
        invalid_module=import_nearest_module_path,
        allowed_dependencies=allowed_dependencies,
        deprecated_dependencies=deprecated_dependencies,
    )


@dataclass
class BoundaryError:
    file_path: Path
    line_number: int
    import_mod_path: str
    error_info: ErrorInfo


@dataclass
class CheckResult:
    errors: list[BoundaryError] = field(default_factory=list)
    deprecated_warnings: list[BoundaryError] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def is_path_excluded(
    path: Path, exclude_paths: list[str], use_regex_matching: bool
) -> bool:
    return any(
        (
            re.match(exclude_path, f"{path}/")
            if use_regex_matching
            else fnmatch.fnmatch(str(path), exclude_path)
        )
        for exclude_path in exclude_paths
    )


def check(
    project_root: Path,
    project_config: ProjectConfig,
    exclude_paths: list[str],
) -> CheckResult:
    if not project_root.is_dir():
        raise errors.TachSetupError(
            f"The path {project_root} is not a valid directory."
        )

    boundary_errors: list[BoundaryError] = []
    boundary_warnings: list[BoundaryError] = []
    warnings: list[str] = []

    source_roots = [
        project_root / source_root for source_root in project_config.source_roots
    ]

    module_validation_result = fs.validate_project_modules(
        source_roots=source_roots, modules=project_config.modules
    )
    warnings.extend(
        f"Module '{module.path}' not found. It will be ignored."
        for module in module_validation_result.invalid_modules
    )
    module_tree = build_module_tree(
        source_roots=source_roots,
        modules=module_validation_result.valid_modules,
        forbid_circular_dependencies=project_config.forbid_circular_dependencies,
    )

    found_at_least_one_project_import = False
    # This informs the Rust extension ahead-of-time which paths are excluded.
    # The extension builds regex/glob patterns and uses them during `get_project_imports`
    set_excluded_paths(
        project_root=str(project_root),
        exclude_paths=exclude_paths,
        use_regex_matching=project_config.use_regex_matching,
    )
    for source_root in source_roots:
        for file_path in fs.walk_pyfiles(source_root):
            abs_file_path = source_root / file_path
            rel_file_path = abs_file_path.relative_to(project_root)
            if is_path_excluded(
                rel_file_path,
                exclude_paths=exclude_paths,
                use_regex_matching=project_config.use_regex_matching,
            ):
                continue

            mod_path = fs.file_to_module_path(
                source_roots=tuple(source_roots), file_path=abs_file_path
            )
            nearest_module = module_tree.find_nearest(mod_path)
            if nearest_module is None:
                continue

            try:
                project_imports = get_project_imports(
                    source_roots=list(map(str, source_roots)),
                    file_path=str(abs_file_path),
                    ignore_type_checking_imports=project_config.ignore_type_checking_imports,
                )
            except SyntaxError:
                warnings.append(f"Skipping '{file_path}' due to a syntax error.")
                continue
            except OSError:
                warnings.append(f"Skipping '{file_path}' due to a file system error.")
                continue
            for project_import in project_imports:
                found_at_least_one_project_import = True
                error_info = check_import(
                    module_tree=module_tree,
                    import_mod_path=project_import[0],
                    file_nearest_module=nearest_module,
                    file_mod_path=mod_path,
                )
                if error_info is None:
                    continue
                boundary_error = BoundaryError(
                    file_path=file_path,
                    import_mod_path=project_import[0],
                    line_number=project_import[1],
                    error_info=error_info,
                )
                if error_info.is_deprecated:
                    boundary_warnings.append(boundary_error)
                else:
                    boundary_errors.append(boundary_error)

    if not found_at_least_one_project_import:
        warnings.append(
            "WARNING: No first-party imports were found. You may need to use 'tach mod' to update your Python source roots. Docs: https://docs.gauge.sh/usage/configuration#source-roots"
        )
    return CheckResult(
        errors=boundary_errors, deprecated_warnings=boundary_warnings, warnings=warnings
    )


__all__ = ["BoundaryError", "check"]
