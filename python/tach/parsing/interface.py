from __future__ import annotations

import ast
from typing import TYPE_CHECKING, Any

from tach import filesystem as fs
from tach.parsing.ast_visitor import EarlyExitNodeVisitor

if TYPE_CHECKING:
    from pathlib import Path


class InterfaceVisitor(EarlyExitNodeVisitor):
    def __init__(self, *args: Any, **kwargs: Any):
        super().__init__(*args, **kwargs)
        self.members: list[str] = []

    def visit_Assign(self, node: ast.Assign):
        # Check if the node is an assignment
        if any(
            isinstance(target, ast.Name) and target.id == "__all__"
            for target in node.targets
        ):
            # Ensure the value is a list and process each element
            if isinstance(node.value, ast.List):
                for element in node.value.elts:
                    if isinstance(element, ast.Str):  # pyright: ignore
                        # Supports Python 3.7 and below
                        self.members.append(element.s)
                    elif isinstance(element, ast.Constant):
                        # Supports Python 3.8 and above
                        if isinstance(element.value, str):
                            self.members.append(element.value)
                # Early exit
                self.set_exit(True)
        # Continue to the next node
        self.generic_visit(node)


def parse_interface_members(source_root: Path, module_path: str) -> list[str]:
    """
    Parse the members of __all__ in a given module
    """
    file_path = fs.module_to_file_path_no_members(source_root, module_path)
    if file_path is None:
        return []

    parsed_ast = fs.parse_ast(str(file_path))
    interface_visitor = InterfaceVisitor()
    interface_visitor.visit(parsed_ast)
    return interface_visitor.members
