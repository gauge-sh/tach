from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from tach import errors
from tach.colors import BCOLORS
from tach.extension import (
    create_dependency_report,
    get_external_imports,
    set_excluded_paths,
)
from tach.filesystem import walk_pyfiles
from tach.utils.display import create_clickable_link

if TYPE_CHECKING:
    from tach.core import ProjectConfig


def report(
    project_root: Path,
    path: Path,
    project_config: ProjectConfig,
    include_dependency_modules: list[str] | None = None,
    include_usage_modules: list[str] | None = None,
    skip_dependencies: bool = False,
    skip_usages: bool = False,
    exclude_paths: list[str] | None = None,
) -> str:
    if not project_root.is_dir():
        raise errors.TachSetupError(
            f"The path '{project_root}' is not a valid directory."
        )

    if not path.exists():
        raise errors.TachError(f"The path '{path}' does not exist.")

    if exclude_paths is not None and project_config.exclude is not None:
        exclude_paths.extend(project_config.exclude)
    else:
        exclude_paths = project_config.exclude

    # This informs the Rust extension ahead-of-time which paths are excluded.
    set_excluded_paths(
        project_root=str(project_root), exclude_paths=exclude_paths or []
    )

    return create_dependency_report(
        project_root=str(project_root),
        source_roots=[
            str(project_root / source_root)
            for source_root in project_config.source_roots
        ],
        path=str(path),
        include_dependency_modules=include_dependency_modules,
        include_usage_modules=include_usage_modules,
        skip_dependencies=skip_dependencies,
        skip_usages=skip_usages,
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )


@dataclass
class ExternalDependency:
    absolute_file_path: Path
    import_module_path: str
    import_line_number: int
    package_name: str


def render_external_dependency(
    dependency: ExternalDependency, display_path: Path
) -> str:
    clickable_link = create_clickable_link(
        file_path=dependency.absolute_file_path,
        display_path=display_path,
        line=dependency.import_line_number,
    )
    return (
        f"{BCOLORS.OKGREEN}{clickable_link}{BCOLORS.ENDC}{BCOLORS.OKCYAN}: "
        f"Import '{dependency.import_module_path}' from package '{dependency.package_name}'{BCOLORS.ENDC}"
    )


def render_external_dependency_report(
    path: Path, dependencies: list[ExternalDependency], raw: bool = False
) -> str:
    if raw:
        return "\n".join(dependency.package_name for dependency in dependencies)

    title = f"[ External Dependencies in '{path}' ]"
    divider = "-" * len(title)
    lines = [title, divider]

    if not dependencies:
        lines.append(f"{BCOLORS.OKGREEN}No external dependencies found.{BCOLORS.ENDC}")
        return "\n".join(lines)

    for dependency in dependencies:
        lines.append(
            render_external_dependency(dependency=dependency, display_path=path)
        )

    return "\n".join(lines)


PYPI_PACKAGE_REGEX = re.compile(r"[-_.]+")


def normalize_package_name(import_module_path: str) -> str:
    package_name = import_module_path.split(".")[0]
    return PYPI_PACKAGE_REGEX.sub("-", package_name).lower()


def get_external_dependencies(
    project_root: str,
    source_roots: list[str],
    file_path: str,
    ignore_type_checking_imports: bool,
) -> list[ExternalDependency]:
    external_imports = get_external_imports(
        project_root=project_root,
        source_roots=source_roots,
        file_path=file_path,
        ignore_type_checking_imports=ignore_type_checking_imports,
    )
    return [
        ExternalDependency(
            absolute_file_path=Path(file_path),
            import_module_path=external_import[0],
            import_line_number=external_import[1],
            package_name=normalize_package_name(external_import[0]),
        )
        for external_import in external_imports
    ]


def external_dependency_report(
    project_root: Path,
    path: Path,
    project_config: ProjectConfig,
    raw: bool = False,
    exclude_paths: list[str] | None = None,
) -> str:
    if not project_root.is_dir():
        raise errors.TachSetupError(
            f"The path '{project_root}' is not a valid directory."
        )

    if not path.exists():
        raise errors.TachError(f"The path '{path}' does not exist.")

    if exclude_paths is not None and project_config.exclude is not None:
        exclude_paths.extend(project_config.exclude)
    else:
        exclude_paths = project_config.exclude

    # This informs the Rust extension ahead-of-time which paths are excluded.
    set_excluded_paths(
        project_root=str(project_root), exclude_paths=exclude_paths or []
    )
    source_roots = [
        str(project_root / source_root) for source_root in project_config.source_roots
    ]

    if path.is_file():
        external_dependencies = get_external_dependencies(
            project_root=str(project_root),
            source_roots=source_roots,
            file_path=str(path.resolve()),
            ignore_type_checking_imports=project_config.ignore_type_checking_imports,
        )
        return render_external_dependency_report(path, external_dependencies, raw=raw)

    all_external_dependencies: list[ExternalDependency] = []
    for pyfile in walk_pyfiles(path):
        all_external_dependencies.extend(
            get_external_dependencies(
                project_root=str(project_root),
                source_roots=source_roots,
                file_path=str(path / pyfile),
                ignore_type_checking_imports=project_config.ignore_type_checking_imports,
            )
        )

    return render_external_dependency_report(path, all_external_dependencies, raw=raw)


__all__ = ["report", "external_dependency_report"]
