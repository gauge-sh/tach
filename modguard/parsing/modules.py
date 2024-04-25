from typing import Optional

from modguard import filesystem as fs
from modguard.core import ModuleConfig
from modguard.core.module import ModuleTrie


def build_module_trie(
    root: str,
    exclude_paths: Optional[list[str]] = None,
) -> ModuleTrie:
    boundary_trie = ModuleTrie()

    for dir_path, config_path in fs.walk_modules(root, exclude_paths=exclude_paths):
        boundary_trie.insert(
            ModuleConfig.from_yml(fs.read_file(config_path)),
            fs.file_to_module_path(dir_path),
        )

    return boundary_trie
