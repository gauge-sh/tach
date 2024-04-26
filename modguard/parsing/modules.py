from typing import Optional

from modguard import filesystem as fs
from modguard.core import ModuleTrie
from modguard.parsing import parse_module_config, parse_interface_members


def build_module_trie(
    root: str,
    exclude_paths: Optional[list[str]] = None,
    ignore_hidden_paths: Optional[bool] = True,
) -> ModuleTrie:
    boundary_trie = ModuleTrie()

    for dir_path in fs.walk_modules(
        root,
        exclude_paths=exclude_paths,
        ignore_hidden_paths=ignore_hidden_paths,
    ):
        module_config = parse_module_config(dir_path)
        if module_config is None:
            raise ValueError(f"Could not parse module config for {dir_path}")
        boundary_trie.insert(
            config=module_config,
            path=fs.file_to_module_path(dir_path),
            interface_members=parse_interface_members(dir_path),
        )

    return boundary_trie
