from __future__ import annotations

from typing import Optional

from tach import filesystem as fs
from tach.core import PackageTrie
from tach.parsing import parse_interface_members, parse_package_config


def build_package_trie(
    root: str,
    exclude_paths: Optional[list[str]] = None,
) -> PackageTrie:
    package_trie = PackageTrie()

    for dir_path, _ in fs.walk_configured_packages(
        root,
        exclude_paths=exclude_paths,
    ):
        package_config = parse_package_config(dir_path)
        if package_config is None:
            raise ValueError(f"Could not parse package config for {dir_path}")
        package_trie.insert(
            config=package_config,
            path=fs.file_to_module_path(dir_path),
            interface_members=parse_interface_members(dir_path),
        )

    return package_trie
