from collections import deque
from dataclasses import dataclass, field
from typing import Optional, Generator

from .public import PublicMember
from modguard.errors import ModguardSetupError, ModguardError


@dataclass
class BoundaryNode:
    public_members: dict[str, PublicMember] = field(default_factory=dict)
    children: dict[str, "BoundaryNode"] = field(default_factory=dict)
    is_end_of_path: bool = False
    full_path: str = ""

    def add_public_member(self, member: PublicMember):
        self.public_members[member.name] = member


@dataclass
class BoundaryTrie:
    root: BoundaryNode = field(default_factory=BoundaryNode)

    def __iter__(self):
        return boundary_trie_iterator(self)

    @staticmethod
    def _split_mod_path(path: str) -> list[str]:
        # By default "".split(".") -> ['']
        # so we want to remove any whitespace path components
        return [part for part in path.split(".") if part]

    def get(self, path: str) -> Optional[BoundaryNode]:
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node if node.is_end_of_path else None

    def insert(self, path: str, public_members: Optional[list[PublicMember]] = None):
        node = self.root
        parts = self._split_mod_path(path)

        for part in parts:
            if part not in node.children:
                node.children[part] = BoundaryNode()
            node = node.children[part]

        if public_members:
            node.public_members = {member.name: member for member in public_members}

        node.is_end_of_path = True
        node.full_path = path

    def register_public_member(self, path: str, member: PublicMember):
        if not path and not member.name:
            raise ModguardError(
                "Registering the root boundary as public (in '__init__.py') is invalid."
            )
        nearest_boundary = self.find_nearest(path)
        if not nearest_boundary:
            raise ModguardSetupError(f"Could not register public member {path}")

        member_path = f"{path}.{member.name}" if member.name else path
        if member_path not in nearest_boundary.public_members:
            nearest_boundary.public_members[member_path] = member

    def find_nearest(self, path: str) -> Optional[BoundaryNode]:
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


def boundary_trie_iterator(trie: BoundaryTrie) -> Generator[BoundaryNode, None, None]:
    stack = deque([trie.root])

    while stack:
        node = stack.popleft()
        if node.is_end_of_path:
            yield node

        stack.extend(node.children.values())
