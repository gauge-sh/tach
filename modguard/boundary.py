from dataclasses import dataclass, field
from typing import Optional

from .visibility import PublicMember
from .errors import ModguardSetupError


@dataclass
class Boundary:
    name: str = ""


@dataclass
class BoundaryNode:
    name: str = ""
    public_members: dict[str, PublicMember] = field(default_factory=dict)
    children: dict = field(default_factory=dict)
    is_end_of_path: bool = False
    full_path: str = None


@dataclass
class BoundaryTrie:
    root: BoundaryNode = field(default_factory=BoundaryNode)

    def get(self, path: str) -> Optional[BoundaryNode]:
        node = self.root
        parts = path.split(".")

        for part in parts:
            if part not in node.children:
                return None
            node = node.children[part]

        return node

    def insert(self, path: str, public_members: dict[str, PublicMember] = None):
        node = self.root
        parts = path.split(".")
        # Don't treat empty string as a path part
        parts = [part for part in parts if part]

        for part in parts:
            if part not in node.children:
                node.children[part] = BoundaryNode()
            node = node.children[part]

        if public_members:
            node.public_members = public_members

        node.is_end_of_path = True
        node.full_path = path

    def register_public_member(self, path: str, member: PublicMember):
        nearest_boundary = self.find_nearest(path)
        if not nearest_boundary:
            raise ModguardSetupError(f"Could not register public member {path}")

        # For empty member.name, implication is that the entire module is public
        member_path = f"{path}.{member.name}" if member.name else path
        if member_path not in nearest_boundary.public_members:
            nearest_boundary.public_members[member_path] = member

    def find_nearest(self, path: str) -> BoundaryNode:
        node = self.root
        parts = path.split(".")
        nearest_parent = node if node.is_end_of_path else None

        for part in parts:
            if part in node.children:
                node = node.children[part]
                if node.is_end_of_path:
                    nearest_parent = node
            else:
                break

        return nearest_parent
