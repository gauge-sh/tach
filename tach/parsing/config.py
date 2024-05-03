from typing import Optional, Any

import yaml

from tach.colors import BCOLORS
from tach.constants import TOOL_NAME
from tach.core import (
    ProjectConfig,
    PackageConfig,
    FullConfig,
    PackageTrie,
    TagDependencyRules,
)
from tach import filesystem as fs
from tach.parsing.interface import parse_interface_members


def dump_project_config_to_yaml(config: ProjectConfig) -> str:
    # Using sort_keys=False here and depending on config.model_dump maintaining 'insertion order'
    # so that 'tag' appears before 'depends_on'
    # Instead, should provide custom yaml.Dumper & yaml.Representer or just write our own
    return yaml.dump(config.model_dump(), sort_keys=False)


def parse_project_config_yml(root: str = ".") -> ProjectConfig:
    file_path = fs.validate_project_config_yml_path(root)
    with open(file_path, "r") as f:
        result = yaml.safe_load(f)
        if not result or not isinstance(result, dict):
            raise ValueError(f"Empty or invalid project config file: {file_path}")
    was_deprecated_config, config = ProjectConfig.factory(result)  # type: ignore
    # Automatically update the config if it used the deprecated format
    if was_deprecated_config:
        print(
            f"{BCOLORS.WARNING} Auto-updating project configuration format.{BCOLORS.ENDC}"
        )
        fs.write_file(file_path, dump_project_config_to_yaml(config))
    return config


def parse_package_config_yml(root: str = ".") -> Optional[PackageConfig]:
    file_path = fs.validate_package_config(root)
    if file_path:
        with open(file_path, "r") as f:
            result = yaml.safe_load(f)
            if not result or not isinstance(result, dict):
                raise ValueError(f"Empty or invalid package config file: {file_path}")
        # We want to error on type issues here for now
        return PackageConfig(**result)  # type: ignore


def build_package_trie_from_yml(
    root: str, project_config: ProjectConfig
) -> PackageTrie:
    package_trie = PackageTrie()

    for dir_path in fs.walk_configured_packages(
        root,
        exclude_paths=project_config.exclude,
        exclude_hidden_paths=project_config.exclude_hidden_paths,
    ):
        package_config = parse_package_config_yml(dir_path)
        if package_config is None:
            raise ValueError(f"Could not parse package config for {dir_path}")
        package_trie.insert(
            config=package_config,
            path=fs.file_to_module_path(dir_path),
            interface_members=parse_interface_members(dir_path),
        )

    return package_trie


def read_toml_file(path: str) -> dict[str, Any]:
    try:
        import tomllib

        with open(path, "rb") as f:
            return tomllib.load(f)
    except ImportError:
        import toml  # pyright: ignore

        with open(path, "r") as f:
            return toml.loads(f.read())


def parse_toml_project_config(toml_config: dict[str, Any]) -> ProjectConfig:
    project_config = ProjectConfig()
    if "exclude" in toml_config:
        project_config.exclude = toml_config["exclude"]
    if "exclude_hidden_paths" in toml_config:
        project_config.exclude_hidden_paths = toml_config["exclude_hidden_paths"]
    if "constraints" in toml_config:
        project_config.constraints = [
            TagDependencyRules(
                tag=constraint["tag"], depends_on=constraint["depends_on"]
            )
            for constraint in toml_config["constraints"]
        ]
    return project_config


def parse_toml_packages(toml_config_packages: list[dict[str, Any]]) -> PackageTrie:
    packages = PackageTrie()
    for package in toml_config_packages:
        packages.insert(
            config=PackageConfig(
                tags=package["tags"], strict=package.get("strict", False)
            ),
            path=fs.file_to_module_path(package["path"]),
            interface_members=parse_interface_members(package["path"]),
        )
    return packages


def toml_root_config_exists(root: str = ".") -> bool:
    config_path = fs.find_toml_config(path=root)
    if not config_path:
        return False

    config = read_toml_file(config_path)

    try:
        return bool(set(config["tool"][TOOL_NAME].keys()) - {"packages"})
    except KeyError:
        return False


def parse_pyproject_toml_packages_only(root: str = ".") -> Optional[FullConfig]:
    config_path = fs.find_toml_config(path=root)
    if not config_path:
        return None

    config = read_toml_file(config_path)

    try:
        if TOOL_NAME in config["tool"]:
            packages = config["tool"][TOOL_NAME]["packages"]
        else:
            packages = config["tool"][f"{TOOL_NAME}.packages"]
    except KeyError:
        return FullConfig(project=ProjectConfig(), packages=PackageTrie())

    return FullConfig(
        project=ProjectConfig(),
        packages=parse_toml_packages(packages),
    )


def parse_pyproject_toml_config(root: str = ".") -> Optional[FullConfig]:
    config_path = fs.find_toml_config(path=root)
    if not config_path:
        return None

    config = read_toml_file(config_path)

    try:
        full_config_dict = config["tool"][TOOL_NAME]
    except KeyError:
        return None

    return FullConfig(
        project=parse_toml_project_config(full_config_dict),
        packages=parse_toml_packages(full_config_dict.get("packages", [])),
    )


__root_project_toml_template = """[tool.{tool_name}]
exclude = [{excludes}]
exclude_hidden_paths = {exclude_hidden_paths}
"""

__constraint_toml_template = """[[tool.{tool_name}.constraints]]
tag = {tag}
depends_on = [{depends_on}]
"""


def dump_project_config_to_toml(project_config: ProjectConfig) -> str:
    root_section = __root_project_toml_template.format(
        tool_name=TOOL_NAME,
        excludes=",".join(
            f'"{exclude_path}"' for exclude_path in project_config.exclude
        ),
        exclude_hidden_paths="true" if project_config.exclude_hidden_paths else "false",
    )
    constraint_sections = "\n".join(
        __constraint_toml_template.format(
            tool_name=TOOL_NAME,
            tag=f'"{constraint.tag}"',
            depends_on=",".join(
                f'"{dependency}"' for dependency in constraint.depends_on
            ),
        )
        for constraint in project_config.constraints
    )
    return root_section + "\n" + constraint_sections


def parse_config(
    root: str = ".", exclude_paths: Optional[list[str]] = None
) -> FullConfig:
    toml_config = parse_pyproject_toml_config(root)
    if not toml_config:
        # If no TOML config present, just parse and build everything from YML
        project_config = parse_project_config_yml(root)
        project_config.merge_exclude_paths(exclude_paths=exclude_paths)
        return FullConfig(
            project=project_config,
            packages=build_package_trie_from_yml(
                root=root, project_config=project_config
            ),
        )

    # If TOML config is present, still parse available YML
    # and overwrite config when it is found
    if fs.get_project_config_yml_path(root):
        project_config_yml = parse_project_config_yml(root)
        toml_config.merge_project_config(project_config=project_config_yml)
    toml_config.merge_exclude_paths(exclude_paths=exclude_paths)
    packages = build_package_trie_from_yml(
        root=root, project_config=toml_config.project
    )
    toml_config.merge_packages(packages)
    return toml_config
