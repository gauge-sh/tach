from typing import Optional

from tach.colors import BCOLORS
from tach.core.base import Config
from tach.core.package import PackageTrie
from tach.core.project import ProjectConfig


class FullConfig(Config):
    """
    Full configuration, including global project-level configuration
    and the package trie.
    """

    project: ProjectConfig
    packages: PackageTrie

    def merge_project_config(self, project_config: ProjectConfig):
        # Overwrite all conflicting attributes
        for attr, value in project_config.model_dump():
            if value != getattr(self.project, attr):
                print(
                    f"{BCOLORS.WARNING} Overwriting {attr} in project configuration.{BCOLORS.ENDC}"
                )
                setattr(self.project, attr, value)

    def merge_exclude_paths(self, exclude_paths: Optional[list[str]] = None):
        self.project.merge_exclude_paths(exclude_paths=exclude_paths)

    def merge_packages(self, packages: PackageTrie):
        # Upsert all new packages
        for package in packages:
            if package.config is None:
                continue
            if self.packages.get(package.full_path) is not None:
                print(
                    f"{BCOLORS.WARNING} Overwriting package configuration at {package.full_path}{BCOLORS.ENDC}"
                )
            self.packages.insert(
                config=package.config,
                path=package.full_path,
                interface_members=package.interface_members,
            )
