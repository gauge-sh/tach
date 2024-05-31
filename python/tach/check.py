from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from tach import errors
from tach import filesystem as fs
from tach.extension import get_project_imports, set_excluded_paths
from tach.parsing.modules import build_module_tree

if TYPE_CHECKING:
    from tach.core import ModuleNode, ModuleTree, ProjectConfig


@dataclass
class ErrorInfo:
    source_module: str = ""
    invalid_module: str = ""
    allowed_modules: list[str] = field(default_factory=list)
    exception_message: str = ""

    @property
    def is_dependency_error(self) -> bool:
        return all((self.source_module, self.invalid_module))


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
                    f"The import '{import_mod_path}' (in '{file_mod_path}') "
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
    dependency_tags = file_nearest_module.config.depends_on
    if any(
        dependency_tag == import_nearest_module_path
        for dependency_tag in dependency_tags
    ):
        # The import matches at least one expected dependency
        return None
    # This means the import is not declared as a dependency of the file
    return ErrorInfo(
        source_module=file_nearest_module_path,
        invalid_module=import_nearest_module_path,
        allowed_modules=dependency_tags,
    )


@dataclass
class BoundaryError:
    file_path: str
    line_number: int
    import_mod_path: str
    error_info: ErrorInfo


def check(
    root: str,
    project_config: ProjectConfig,
    exclude_paths: list[str] | None = None,
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

        module_tree = build_module_tree(project_config.modules)

        # This informs the Rust extension ahead-of-time which paths are excluded.
        # The extension builds regexes and uses them during `get_project_imports`
        set_excluded_paths(exclude_paths=exclude_paths or [])
        boundary_errors: list[BoundaryError] = []
        for file_path in fs.walk_pyfiles(
            ".",
            exclude_paths=exclude_paths,
        ):
            mod_path = fs.file_to_module_path(file_path)
            nearest_module = module_tree.find_nearest(mod_path)
            if nearest_module is None:
                continue

            project_imports = get_project_imports(
                ".",
                file_path,
                ignore_type_checking_imports=project_config.ignore_type_checking_imports,
            )
            for project_import in project_imports:
                check_error = check_import(
                    module_tree=module_tree,
                    import_mod_path=project_import[0],
                    file_nearest_module=nearest_module,
                    file_mod_path=mod_path,
                )
                if check_error is None:
                    continue

                boundary_errors.append(
                    BoundaryError(
                        file_path=file_path,
                        import_mod_path=project_import[0],
                        line_number=project_import[1],
                        error_info=check_error,
                    )
                )

        return boundary_errors
    finally:
        fs.chdir(cwd)
