from typing import Optional

import yaml

from tach.colors import BCOLORS
from tach.core import ProjectConfig, PackageConfig, FullConfig, PackageTrie
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


def parse_pyproject_toml_config(root: str = ".") -> Optional[FullConfig]: ...


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
    project_config_yml = parse_project_config_yml(root)
    toml_config.merge_project_config(project_config=project_config_yml)
    toml_config.merge_exclude_paths(exclude_paths=exclude_paths)
    packages = build_package_trie_from_yml(
        root=root, project_config=toml_config.project
    )
    toml_config.merge_packages(packages)
    return toml_config
