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

    @classmethod
    def from_packages_only(cls, packages: PackageTrie) -> "FullConfig":
        # This is useful in init, since we initialize the root configuration _after_
        # the package configuration
        return cls(project=ProjectConfig(), packages=packages)

    def merge_project_config(self, project_config: ProjectConfig):
        # Overwrite all conflicting attributes
        for attr in project_config.model_dump().keys():
            if getattr(project_config, attr) != getattr(self.project, attr):
                print(
                    f"{BCOLORS.WARNING} Overwriting {attr} in project configuration.{BCOLORS.ENDC}"
                )
                setattr(self.project, attr, getattr(project_config, attr))

    def merge_exclude_paths(self, exclude_paths: Optional[list[str]] = None):
        self.project.merge_exclude_paths(exclude_paths=exclude_paths)

    def merge_packages(self, packages: PackageTrie):
        # Upsert all new packages
        for package in packages:
            if package.config is None:
                continue
            existing_package = self.packages.get(package.full_path)
            if existing_package is not None and existing_package != package:
                print(
                    f"{BCOLORS.WARNING} Overwriting package configuration at {package.full_path}{BCOLORS.ENDC}"
                )
            self.packages.insert(
                config=package.config,
                path=package.full_path,
                interface_members=package.interface_members,
            )
