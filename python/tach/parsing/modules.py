from __future__ import annotations

from typing import TYPE_CHECKING

from tach.core import ModuleTree
from tach.parsing import parse_interface_members

if TYPE_CHECKING:
    from tach.core import ModuleConfig


def build_module_tree(modules: list[ModuleConfig]) -> ModuleTree:
    tree = ModuleTree()
    for module in modules:
        tree.insert(
            config=module,
            path=module.mod_path,
            interface_members=parse_interface_members(module.path),
        )
    return tree
