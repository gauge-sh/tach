from typing import Optional

import yaml

from tach.colors import BCOLORS
from tach.core import ProjectConfig, PackageConfig
from tach import filesystem as fs


def dump_project_config_to_yaml(config: ProjectConfig) -> str:
    # Using sort_keys=False here and depending on config.model_dump maintaining 'insertion order'
    # so that 'tag' appears before 'depends_on'
    # Instead, should provide custom yaml.Dumper & yaml.Representer or just write our own
    return yaml.dump(config.model_dump(), sort_keys=False)


def parse_project_config(root: str = ".") -> ProjectConfig:
    file_path = fs.validate_project_config_path(root)
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


def parse_package_config(root: str = ".") -> Optional[PackageConfig]:
    file_path = fs.validate_package_config(root)
    if file_path:
        with open(file_path, "r") as f:
            result = yaml.safe_load(f)
            if not result or not isinstance(result, dict):
                raise ValueError(f"Empty or invalid package config file: {file_path}")
        # We want to error on type issues here for now
        return PackageConfig(**result)  # type: ignore
