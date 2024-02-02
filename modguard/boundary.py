from dataclasses import dataclass, field


@dataclass
class Boundary:
    name: str = ""


@dataclass
class BoundaryNode:
    name: str = ""
    public_members: list[str] = field(default_factory=list)
    children: dict = field(default_factory=dict)
    is_end_of_path: bool = False
    full_path: str = None


@dataclass
class BoundaryTrie:
    root: BoundaryNode = field(default_factory=BoundaryNode)

    def insert(self, path: str):
        node = self.root
        parts = path.split(".")

        for part in parts:
            if part not in node.children:
                node.children[part] = BoundaryNode()
            node = node.children[part]

        node.is_end_of_path = True
        node.full_path = path

    def find_nearest(self, path: str) -> str:
        node = self.root
        parts = path.split(".")
        nearest_parent_path = None

        for part in parts:
            if part in node.children:
                node = node.children[part]
                if node.is_end_of_path:
                    nearest_parent_path = node.full_path  # Update nearest parent path
            else:
                break  # No further matching part in the trie

        return nearest_parent_path
