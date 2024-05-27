from __future__ import annotations

import ast
import os
from typing import Any

from tach import filesystem as fs
from tach.parsing.ast_visitor import EarlyExitNodeVisitor


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


def parse_interface_members(path: str) -> list[str]:
    """
    Parse the members of __all__ in the __init__.py of a package.
    'path' should be a path to the package directory.
    """
    init_py_path = os.path.join(path, "__init__.py")
    if not os.path.exists(init_py_path):
        raise ValueError(
            f"Could not parse interface from path, no __init__.py found: {path}"
        )
    parsed_ast = fs.parse_ast(init_py_path)
    interface_visitor = InterfaceVisitor()
    interface_visitor.visit(parsed_ast)
    return interface_visitor.members
