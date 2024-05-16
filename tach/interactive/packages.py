import os
from collections import deque
from dataclasses import dataclass, field
from itertools import chain
from typing import Optional, Generator

from prompt_toolkit import ANSI
from rich.console import Console
from rich.tree import Tree
from rich.text import Text
from prompt_toolkit.application import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import Layout, HSplit, Window
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.widgets import Frame
from prompt_toolkit.styles import Style

from tach import errors
from tach import filesystem as fs
from tach.constants import PACKAGE_FILE_NAME


@dataclass
class FileNode:
    full_path: str
    is_dir: bool
    is_package: bool = False
    parent: Optional["FileNode"] = None
    children: list["FileNode"] = field(default_factory=list)

    @classmethod
    def build_from_path(cls, path: str) -> "FileNode":
        is_dir = os.path.isdir(path)
        is_package = os.path.isfile(os.path.join(path, f"{PACKAGE_FILE_NAME}.yml"))
        return cls(full_path=path, is_dir=is_dir, is_package=is_package)


@dataclass
class FileTree:
    root: FileNode
    nodes: dict[str, FileNode] = field(default_factory=dict)

    @classmethod
    def build_from_path(cls, path: str, depth: int = 1) -> "FileTree":
        root = FileNode.build_from_path(fs.canonical(path))
        tree = cls(root)
        tree._build_subtree(root, depth)
        return tree

    def _build_subtree(self, root: FileNode, depth: int):
        if depth <= 0:
            return

        if root.is_dir:
            try:
                for entry in os.listdir(root.full_path):
                    if entry.startswith("."):
                        # Ignore hidden files and directories
                        continue
                    entry_path = os.path.join(root.full_path, entry)
                    if not os.path.isdir(entry_path):
                        # Only interested in directories for now
                        continue
                    child_node = FileNode.build_from_path(entry_path)
                    child_node.parent = root
                    root.children.append(child_node)
                    self.nodes[entry_path] = child_node
                    if child_node.is_dir:
                        self._build_subtree(child_node, depth - 1)
            except PermissionError:
                # This is expected to occur during listdir when the directory cannot be accessed
                # We simply bail if that happens, meaning it won't show up in the interactive viewer
                return

    def expand_path(self, path: str):
        if path not in self.nodes:
            raise errors.TachError(f"Directory {path} not found in tree.")
        node = self.nodes[path]
        if not node.is_dir:
            raise errors.TachError(
                f"{path} does not seem to be a directory and cannot be expanded."
            )

        self._build_subtree(node, depth=1)

    def __iter__(self):
        return file_tree_iterator(self)


def file_tree_iterator(tree: FileTree) -> Generator[FileNode, None, None]:
    # DFS traversal for printing
    stack = deque([tree.root])

    while stack:
        node = stack.popleft()
        yield node
        stack.extendleft(sorted(node.children, key=lambda n: n.full_path, reverse=True))


class InteractivePackageTree:
    def __init__(self, path: str, depth: int = 1):
        self.file_tree = FileTree.build_from_path(path=path, depth=depth)
        self.console = Console()
        self.tree_control = FormattedTextControl(text=ANSI(self._render_tree()))
        self.footer_control = self._build_footer()
        self.layout = Layout(
            HSplit([Frame(Window(self.tree_control)), Window(self.footer_control)])
        )
        self.key_bindings = KeyBindings()
        self._register_keybindings()
        self.styles = self._build_styles()
        self.app = Application(
            layout=self.layout,
            key_bindings=self.key_bindings,
            full_screen=True,
            style=self.styles,
        )

    @staticmethod
    def _build_styles() -> Style:
        return Style.from_dict(
            {
                "footer-key": "bold cyan",
            }
        )

    KEY_BINDING_LEGEND = [
        ("Ctrl + c", "Exit without saving"),
        ("Ctrl + s", "Save packages"),
        ("Enter", "Mark/unmark package"),
        ("Up/Down", "Navigate"),
        ("Right", "Expand"),
        ("Left", "Collapse"),
    ]

    @staticmethod
    def _key_binding_text(binding: str, description: str) -> list[tuple[str, str]]:
        return [("class:footer-key", binding), ("", f": {description}  ")]

    @classmethod
    def _build_footer(cls) -> FormattedTextControl:
        footer_text = list(
            chain(
                *(
                    cls._key_binding_text(legend[0], legend[1])
                    for legend in cls.KEY_BINDING_LEGEND
                )
            )
        )
        return FormattedTextControl(text=footer_text)

    def _register_keybindings(self):
        if self.key_bindings.bindings:
            return

        @self.key_bindings.add("c-c")
        def _(event):
            self.app.exit()

        @self.key_bindings.add("c-s")
        def save(event):
            # TODO: save
            self.app.exit()

        @self.key_bindings.add("up")
        def up(event): ...

        @self.key_bindings.add("down")
        def down(event): ...

        @self.key_bindings.add("right")
        def right(event): ...

        @self.key_bindings.add("left")
        def left(event): ...

        @self.key_bindings.add("enter")
        def toggle_package(event): ...

    @staticmethod
    def _render_node(node: FileNode) -> Text:
        basename = os.path.basename(node.full_path)
        if node.is_package:
            return Text(f"[Package] {basename}", style="bold yellow")
        return Text(basename)

    def _render_tree(self):
        tree_root = Tree("Packages")
        # Mapping FileNode paths to rich.Tree branches
        # so that we can iterate over the FileTree and use the
        # parent pointers to find the parent rich.Tree branches
        tree_mapping: dict[str, Tree] = {}

        for node in self.file_tree:
            if node.parent is None:
                # If no parent on FileNode, add to rich.Tree root
                tree_node = tree_root.add(self._render_node(node))
            else:
                if node.parent.full_path not in tree_mapping:
                    raise errors.TachError("Failed to render package tree.")
                # Find parent rich.Tree branch,
                # attach this FileNode to the parent's branch
                parent_tree_node = tree_mapping[node.parent.full_path]
                tree_node = parent_tree_node.add(self._render_node(node))

            # Add this new FileNode to the mapping
            tree_mapping[node.full_path] = tree_node

        with self.console.capture() as capture:
            self.console.print(tree_root)
        return capture.get()

    def _update_display(self):
        self.tree_control.text = ANSI(self._render_tree())

    def run(self):
        self.app.run()


if __name__ == "__main__":
    ipt = InteractivePackageTree(path=".", depth=1)
    ipt.run()
