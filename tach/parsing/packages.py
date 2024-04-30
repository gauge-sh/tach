from typing import Optional

from tach import filesystem as fs
from tach.core import PackageTrie
from tach.parsing import parse_package_config, parse_interface_members


def build_package_trie(
    root: str,
    exclude_paths: Optional[list[str]] = None,
    exclude_hidden_paths: Optional[bool] = True,
) -> PackageTrie:
    package_trie = PackageTrie()

    for dir_path in fs.walk_configured_packages(
        root,
        exclude_paths=exclude_paths,
        exclude_hidden_paths=exclude_hidden_paths,
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
