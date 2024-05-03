import os
from dataclasses import field, dataclass
from typing import Optional


from tach import errors
from tach import filesystem as fs
from tach.check import check
from tach.colors import BCOLORS
from tach.constants import (
    PACKAGE_FILE_NAME,
    CONFIG_FILE_NAME,
    TOOL_NAME,
    TOML_CONFIG_FILE_NAME,
)
from tach.core import ProjectConfig, FullConfig
from tach.parsing import (
    dump_project_config_to_yaml,
    parse_pyproject_toml_config,
    dump_project_config_to_toml,
    build_package_trie_from_yml,
    toml_root_config_exists,
    parse_pyproject_toml_packages_only,
)

__package_yml_template = """tags: ['{dir_name}']\n"""

__package_toml_template = """[[tool.{tool_name}.packages]]
path = "{package_path}"
tags = ["{tag}"]
"""


@dataclass
class PackageInitResult:
    package_paths: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def create_package_yml(package_path: str) -> Optional[str]:
    package_yml_path = os.path.join(package_path, f"{PACKAGE_FILE_NAME}.yml")
    if os.path.exists(package_yml_path):
        return f"{BCOLORS.OKCYAN}Package file '{package_yml_path}' already exists.{BCOLORS.ENDC}"
    package_yml_content = __package_yml_template.format(
        dir_name=package_path.replace(os.path.sep, ".")
    )
    fs.write_file(package_yml_path, package_yml_content)


def create_package_toml(root: str, package_path: str) -> Optional[str]:
    toml_config_path = fs.find_toml_config(path=root)
    if not toml_config_path:
        raise errors.TachError(
            f"Could not find pyproject.toml config in any parent of '{root}'"
        )

    toml_config = parse_pyproject_toml_config()
    if toml_config and toml_config.packages.get(fs.file_to_module_path(package_path)):
        return f"{BCOLORS.OKCYAN}Package '{package_path}' already registered.{BCOLORS.ENDC}"
    package_toml_content = __package_toml_template.format(
        tool_name=TOOL_NAME,
        package_path=package_path,
        tag=package_path.replace(os.path.sep, "."),
    )
    fs.append_to_toml(toml_config_path, package_toml_content)


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


def root_yml_exists(root: str) -> bool:
    return bool(fs.get_project_config_yml_path(root))


def create_root_yml(root: str, project_config: ProjectConfig):
    tach_yml_path = os.path.join(root, f"{CONFIG_FILE_NAME}.yml")
    tach_yml_content = dump_project_config_to_yaml(project_config)
    fs.write_file(tach_yml_path, tach_yml_content)


def root_toml_exists(root: str) -> bool:
    return toml_root_config_exists(root)


def create_root_toml(root: str, project_config: ProjectConfig):
    config_path = fs.find_toml_config(root)
    if not config_path:
        raise errors.TachError(
            f"Could not find pyproject.toml config in any parent of '{root}'"
        )
    fs.append_to_toml(config_path, dump_project_config_to_toml(project_config))


def init_root(
    root: str, exclude_paths: Optional[list[str]] = None, use_toml_config: bool = False
) -> InitRootResult:
    if root_yml_exists(root):
        return InitRootResult(
            warnings=[
                f"{BCOLORS.OKCYAN}Project already contains {CONFIG_FILE_NAME}.yml{BCOLORS.ENDC}"
            ]
        )
    elif root_toml_exists(root):
        return InitRootResult(
            warnings=[
                f"{BCOLORS.OKCYAN}Project already contains configuration for {TOOL_NAME} in "
                f"{TOML_CONFIG_FILE_NAME}{BCOLORS.ENDC}"
            ]
        )

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
        raise errors.TachSetupError(f"The path {root} is not a directory.")

    if exclude_paths is None:
        exclude_paths = ["tests/", "docs/"]

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
