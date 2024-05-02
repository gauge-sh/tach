from typing import Optional

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

    def merge_project_config(self, project_config: ProjectConfig): ...

    def merge_exclude_paths(self, exclude_paths: Optional[list[str]] = None):
        self.project.merge_exclude_paths(exclude_paths=exclude_paths)

    def merge_packages(self, packages: PackageTrie): ...
