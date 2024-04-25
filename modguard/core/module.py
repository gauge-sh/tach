from collections import deque
from dataclasses import dataclass, field
from typing import Optional, Generator

from modguard.core.config import ModuleConfig


@dataclass
class ModuleNode:
    """
    A node in the module trie.

    If 'is_end_of_path' is True, this node represents a module in the project,
    and must have 'config' and 'full_path' set.

    If 'is_end_of_path' is False, this node does not represent a real module,
    and must have 'config' None and 'full_path' as the empty string.
    """

    is_end_of_path: bool
    full_path: str
    config: Optional[ModuleConfig]
    children: dict[str, "ModuleNode"] = field(default_factory=dict)

    @classmethod
    def empty(cls) -> "ModuleNode":
        return ModuleNode(is_end_of_path=False, full_path="", config=None)

    @classmethod
    def build(cls, config: ModuleConfig, full_path: str) -> "ModuleNode":
        return ModuleNode(is_end_of_path=True, full_path=full_path, config=config)

    def fill(self, config: ModuleConfig, full_path: str):
        self.is_end_of_path = True
        self.config = config
        self.full_path = full_path


@dataclass
class ModuleTrie:
    """
    The core data structure for modguard, representing the modules in a project
    with a trie structure for module prefix lookups.
    """

    root: ModuleNode = field(default_factory=ModuleNode.empty)

    def __iter__(self):
        return module_trie_iterator(self)

    @staticmethod
    def _split_mod_path(path: str) -> list[str]:
        # By default "".split(".") -> ['']
        # so we want to remove any whitespace path components
        return [part for part in path.split(".") if part]

    def get(self, path: str) -> Optional[ModuleNode]:
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node if node.is_end_of_path else None

    def insert(self, config: ModuleConfig, path: str):
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                node.children[part] = ModuleNode.empty()
            node = node.children[part]

        node.fill(config, path)

    def find_nearest(self, path: str) -> Optional[ModuleNode]:
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


def module_trie_iterator(trie: ModuleTrie) -> Generator[ModuleNode, None, None]:
    stack = deque([trie.root])

    while stack:
        node = stack.popleft()
        if node.is_end_of_path:
            yield node

        stack.extend(node.children.values())
