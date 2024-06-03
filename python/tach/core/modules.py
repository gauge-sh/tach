from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import Generator

from tach.core.config import ModuleConfig, RootModuleConfig


@dataclass
class ModuleNode:
    """
    A node in the module tree.

    If 'is_end_of_path' is True, this node represents a module in the project,
    and must have 'config' and 'full_path' set.

    If 'is_end_of_path' is False, this node does not represent a real module,
    and must have 'config' None and 'full_path' as the empty string.
    """

    is_end_of_path: bool
    full_path: str
    config: ModuleConfig | None
    interface_members: list[str] = field(default_factory=list)
    children: dict[str, ModuleNode] = field(default_factory=dict)

    @classmethod
    def empty(cls) -> ModuleNode:
        return ModuleNode(is_end_of_path=False, full_path="", config=None)

    @classmethod
    def implicit_root(cls) -> ModuleNode:
        config = RootModuleConfig()
        return ModuleNode(is_end_of_path=True, full_path=".", config=config)

    def fill(
        self, config: ModuleConfig, full_path: str, interface_members: list[str]
    ) -> None:
        self.is_end_of_path = True
        self.config = config
        self.full_path = full_path
        self.interface_members = interface_members


def split_module_path(path: str) -> list[str]:
    if not path or path == ".":
        return []
    return path.split(".")


@dataclass
class ModuleTree:
    """
    The core data structure for tach, representing the modules in a project
    with a tree structure for module path lookups.
    """

    root: ModuleNode = field(default_factory=ModuleNode.implicit_root)

    def __iter__(self):
        return module_tree_iterator(self)

    def get(self, path: str) -> ModuleNode | None:
        if not path:
            return None

        node = self.root
        parts = split_module_path(path)

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node if node.is_end_of_path else None

    def insert(self, config: ModuleConfig, path: str, interface_members: list[str]):
        if not path:
            raise ValueError("Cannot insert module with empty path.")

        node = self.root
        parts = split_module_path(path)

        for part in parts:
            if part not in node.children:
                node.children[part] = ModuleNode.empty()
            node = node.children[part]

        node.fill(config, path, interface_members)

    def find_nearest(self, path: str) -> ModuleNode | None:
        node = self.root
        parts = split_module_path(path)
        nearest_parent = node

        for part in parts:
            if part in node.children:
                node = node.children[part]
                if node.is_end_of_path:
                    nearest_parent = node
            else:
                break

        return nearest_parent if nearest_parent.is_end_of_path else None


def module_tree_iterator(tree: ModuleTree) -> Generator[ModuleNode, None, None]:
    stack = deque([tree.root])

    while stack:
        node = stack.popleft()
        if node.is_end_of_path:
            yield node

        stack.extend(node.children.values())
