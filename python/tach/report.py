from __future__ import annotations

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
from tach.utils.exclude import is_path_excluded
from tach.utils.external import (
    get_package_name,
    is_stdlib_module,
    normalize_package_name,
)

if TYPE_CHECKING:
    from tach.extension import ProjectConfig


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

    # This informs the Rust extension ahead-of-time which paths are excluded.
    set_excluded_paths(
        project_root=str(project_root),
        exclude_paths=exclude_paths or [],
        use_regex_matching=project_config.use_regex_matching,
    )

    try:
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
    except ValueError as e:
        raise errors.TachError(str(e))


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
    if not dependencies:
        if raw:
            return ""
        return f"{BCOLORS.OKCYAN}No external dependencies found in {BCOLORS.ENDC}{BCOLORS.OKGREEN}'{path}'.{BCOLORS.ENDC}"

    if raw:
        return "\n".join({dependency.package_name for dependency in dependencies})

    title = f"[ External Dependencies in '{path}' ]"
    divider = "-" * len(title)
    lines = [title, divider]

    if not dependencies:
        lines.append(f"{BCOLORS.OKGREEN}No external dependencies found.{BCOLORS.ENDC}")
        return "\n".join(lines)

    for dependency in dependencies:
        lines.append(
            render_external_dependency(
                dependency=dependency,
                display_path=dependency.absolute_file_path.relative_to(Path.cwd()),
            )
        )

    return "\n".join(lines)


def get_external_dependencies(
    source_roots: list[str],
    file_path: str,
    ignore_type_checking_imports: bool,
    excluded_modules: set[str] | None = None,
) -> list[ExternalDependency]:
    external_imports = get_external_imports(
        source_roots=source_roots,
        file_path=file_path,
        ignore_type_checking_imports=ignore_type_checking_imports,
    )

    excluded_modules = excluded_modules or set()
    external_dependencies: list[ExternalDependency] = []
    for external_import in external_imports:
        external_package = get_package_name(external_import[0])
        if external_package in excluded_modules:
            continue

        if is_stdlib_module(external_package):
            continue

        external_dependencies.append(
            ExternalDependency(
                absolute_file_path=Path(file_path),
                import_module_path=external_import[0],
                import_line_number=external_import[1],
                package_name=normalize_package_name(external_import[0]),
            )
        )
    return external_dependencies


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

    if exclude_paths and is_path_excluded(
        exclude_paths,
        path,
        use_regex_matching=project_config.use_regex_matching,
    ):
        raise errors.TachError(f"The path '{path}' is excluded.")

    # This informs the Rust extension ahead-of-time which paths are excluded.
    set_excluded_paths(
        project_root=str(project_root),
        exclude_paths=exclude_paths or [],
        use_regex_matching=project_config.use_regex_matching,
    )
    source_roots = [
        str(project_root / source_root) for source_root in project_config.source_roots
    ]

    if path.is_file():
        external_dependencies = get_external_dependencies(
            source_roots=source_roots,
            file_path=str(path.resolve()),
            excluded_modules=set(project_config.external.exclude),
            ignore_type_checking_imports=project_config.ignore_type_checking_imports,
        )
        return render_external_dependency_report(path, external_dependencies, raw=raw)

    all_external_dependencies: list[ExternalDependency] = []
    for pyfile in walk_pyfiles(
        path,
        project_root=project_root,
        exclude_paths=exclude_paths,
        use_regex_matching=project_config.use_regex_matching,
    ):
        all_external_dependencies.extend(
            get_external_dependencies(
                source_roots=source_roots,
                file_path=str(path.resolve() / pyfile),
                excluded_modules=set(project_config.external.exclude),
                ignore_type_checking_imports=project_config.ignore_type_checking_imports,
            )
        )

    return render_external_dependency_report(path, all_external_dependencies, raw=raw)


__all__ = ["report", "external_dependency_report"]
