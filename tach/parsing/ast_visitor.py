from __future__ import annotations

import ast
from typing import Any


class EarlyExitNodeVisitor(ast.NodeVisitor):
    def __init__(self, *args: list[Any], **kwargs: dict[Any, Any]):
        self._exit = False

    def set_exit(self, flag: bool = True):
        self._exit = flag

    def visit(self, node: ast.AST):
        if self._exit:
            # Allow reusing the instance
            self._exit = False
            return
        super().visit(node)
