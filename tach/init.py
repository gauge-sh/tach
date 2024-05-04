import os
from dataclasses import field, dataclass
from typing import Optional


from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.constants import (
    CONFIG_FILE_NAME,
    TOOL_NAME,
    TOML_CONFIG_FILE_NAME,
)
from tach.core import ProjectConfig, FullConfig
from tach.parsing import (
    build_package_trie_from_yml,
    toml_root_config_exists,
    parse_pyproject_toml_packages_only,
    create_root_yml,
    create_root_toml,
    create_package_yml,
    create_package_toml,
)


@dataclass
class PackageInitResult:
    package_paths: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def init_packages(
    root: str,
    depth: int,
    exclude_paths: Optional[list[str]] = None,
    use_toml_config: bool = False,
) -> PackageInitResult:
    package_paths: list[str] = []
    warnings: list[str] = []
    for dir_path in fs.walk_pypackages(root, depth=depth, exclude_paths=exclude_paths):
        if not use_toml_config:
            warning = create_package_yml(package_path=dir_path)
        else:
            warning = create_package_toml(root, package_path=dir_path)

        if warning:
            warnings.append(warning)
        package_paths.append(dir_path)

    return PackageInitResult(package_paths=package_paths, warnings=warnings)


@dataclass
class InitRootResult:
    warnings: list[str] = field(default_factory=list)


def init_root(
    root: str, exclude_paths: Optional[list[str]] = None, use_toml_config: bool = False
) -> InitRootResult:
    # Need to use lower-level methods to parse config
    # since root configuration doesn't exist yet
    if use_toml_config:
        config = parse_pyproject_toml_packages_only(root=root)
        if config is None:
            raise errors.TachError(f"Could not parse pyproject.toml config from {root}")
        config.project.exclude = exclude_paths or []
    else:
        project_config = ProjectConfig()
        project_config.exclude = exclude_paths or []
        config = FullConfig.from_packages_only(
            packages=build_package_trie_from_yml(
                root=root, project_config=project_config
            )
        )

    check_errors = check(root, config=config)
    for error in check_errors:
        if error.is_tag_error:
            config.project.add_dependencies_to_tag(error.source_tag, error.invalid_tags)

    if use_toml_config:
        create_root_toml(root, project_config=config.project)
    else:
        create_root_yml(root, project_config=config.project)

    # Relies on mutation
    check_errors = check(root, config=config)
    if check_errors:
        return InitRootResult(
            warnings=[
                "Could not auto-detect all dependencies, use 'tach check' to finish initialization manually."
            ]
        )

    return InitRootResult(warnings=[])


def init_project(
    root: str,
    depth: Optional[int] = None,
    exclude_paths: Optional[list[str]] = None,
    use_toml_config: bool = False,
) -> list[str]:
    if not os.path.isdir(root):
        raise errors.TachInitError(f"The path {root} is not a directory.")

    if exclude_paths is None:
        exclude_paths = ["tests/", "docs/"]

    if bool(fs.get_project_config_yml_path(root)):
        raise errors.TachInitError(f"Project already contains {CONFIG_FILE_NAME}.yml")
    elif toml_root_config_exists(root):
        raise errors.TachInitError(
            f"Project already contains configuration for {TOOL_NAME} in "
            f"{TOML_CONFIG_FILE_NAME}"
        )

    warnings: list[str] = []

    if depth is None:
        package_init_result = init_packages(
            root, depth=1, exclude_paths=exclude_paths, use_toml_config=use_toml_config
        )
        warnings.extend(package_init_result.warnings)
        if len(package_init_result.package_paths) == 1:
            result = init_packages(
                package_init_result.package_paths[0],
                depth=1,
                exclude_paths=exclude_paths,
                use_toml_config=use_toml_config,
            )
            warnings.extend(result.warnings)
    else:
        package_init_result = init_packages(
            root,
            depth=depth,
            exclude_paths=exclude_paths,
            use_toml_config=use_toml_config,
        )
        warnings.extend(package_init_result.warnings)

    init_root_result = init_root(
        root, exclude_paths=exclude_paths, use_toml_config=use_toml_config
    )
    warnings.extend(init_root_result.warnings)

    return warnings
