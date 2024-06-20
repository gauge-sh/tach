from __future__ import annotations

from typing import TYPE_CHECKING

from tach.core import ModuleTree
from tach.parsing import parse_interface_members

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ModuleConfig


def find_duplicate_modules(modules: list[ModuleConfig]) -> list[str]:
    duplicate_module_paths: list[str] = []
    seen: set[str] = set()
    for module in modules:
        if module.path in seen:
            duplicate_module_paths.append(module.path)
            continue
        seen.add(module.path)
    return duplicate_module_paths


def build_module_tree(source_root: Path, modules: list[ModuleConfig]) -> ModuleTree:
    duplicate_modules = find_duplicate_modules(modules)
    if duplicate_modules:
        raise ValueError(
            f"Failed to build module tree. The following modules were defined more than once: {duplicate_modules}"
        )
    tree = ModuleTree()
    for module in modules:
        tree.insert(
            config=module,
            path=module.mod_path,
            interface_members=parse_interface_members(
                source_root=source_root, module_path=module.path
            ),
        )
    return tree
