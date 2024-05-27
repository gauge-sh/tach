from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import Generator, Optional

from tach.core.config import PackageConfig


@dataclass
class PackageNode:
    """
    A node in the package trie.

    If 'is_end_of_path' is True, this node represents a package in the project,
    and must have 'config' and 'full_path' set.

    If 'is_end_of_path' is False, this node does not represent a real package,
    and must have 'config' None and 'full_path' as the empty string.
    """

    is_end_of_path: bool
    full_path: str
    config: Optional[PackageConfig]
    interface_members: list[str] = field(default_factory=list)
    children: dict[str, "PackageNode"] = field(default_factory=dict)

    @classmethod
    def empty(cls) -> "PackageNode":
        return PackageNode(is_end_of_path=False, full_path="", config=None)

    def fill(
        self, config: PackageConfig, full_path: str, interface_members: list[str]
    ) -> None:
        self.is_end_of_path = True
        self.config = config
        self.full_path = full_path
        self.interface_members = interface_members


@dataclass
class PackageTrie:
    """
    The core data structure for tach, representing the packages in a project
    with a trie structure for package prefix lookups.
    """

    root: PackageNode = field(default_factory=PackageNode.empty)

    def __iter__(self):
        return package_trie_iterator(self)

    @staticmethod
    def _split_mod_path(path: str) -> list[str]:
        # By default "".split(".") -> ['']
        # so we want to remove any whitespace path components
        return [part for part in path.split(".") if part]

    def get(self, path: str) -> Optional[PackageNode]:
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node if node.is_end_of_path else None

    def insert(self, config: PackageConfig, path: str, interface_members: list[str]):
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                node.children[part] = PackageNode.empty()
            node = node.children[part]

        node.fill(config, path, interface_members)

    def find_nearest(self, path: str) -> Optional[PackageNode]:
        node = self.root
        parts = self._split_mod_path(path)
        nearest_parent = node

        for part in parts:
            if part in node.children:
                node = node.children[part]
                if node.is_end_of_path:
                    nearest_parent = node
            else:
                break

        return nearest_parent if nearest_parent.is_end_of_path else None


def package_trie_iterator(trie: PackageTrie) -> Generator[PackageNode, None, None]:
    stack = deque([trie.root])

    while stack:
        node = stack.popleft()
        if node.is_end_of_path:
            yield node

        stack.extend(node.children.values())
