from __future__ import annotations

import ast
import os
import re
from dataclasses import dataclass, field
from typing import Optional

from tach import filesystem as fs


@dataclass
class IgnoreDirective:
    lineno: int
    modules: list[str] = field(default_factory=list)


TACH_IGNORE_REGEX = re.compile(r"# *tach-ignore(( [\w.]+)*)$")


def get_ignore_directives(file_content: str) -> dict[int, IgnoreDirective]:
    ignores: dict[int, IgnoreDirective] = {}
    lines = file_content.splitlines()
    for lineno, line in enumerate(lines):
        normal_lineno = lineno + 1
        match = TACH_IGNORE_REGEX.match(line)
        if match:
            ignored_modules = match.group(1)
            if ignored_modules:
                ignores[normal_lineno] = IgnoreDirective(
                    lineno=lineno + 1, modules=ignored_modules.split()
                )
            else:
                ignores[normal_lineno] = IgnoreDirective(lineno=lineno + 1)
    return ignores


@dataclass
class ProjectImport:
    mod_path: str
    line_number: int


class ImportVisitor(ast.NodeVisitor):
    def __init__(
        self,
        project_root: str,
        current_mod_path: str,
        is_package: bool = False,
        ignore_directives: Optional[dict[int, IgnoreDirective]] = None,
        ignore_type_checking_imports: bool = False,
    ):
        self.project_root = project_root
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.ignored_imports = ignore_directives or {}
        self.imports: list[ProjectImport] = []
        self.ignore_type_checking_imports = ignore_type_checking_imports

    def _get_ignored_modules(self, lineno: int) -> Optional[list[str]]:
        # Check for ignore directive at the previous line or on the current line
        directive = self.ignored_imports.get(lineno - 1) or self.ignored_imports.get(
            lineno
        )
        return directive.modules if directive else None

    def visit_If(self, node: ast.If):
        if isinstance(node.test, ast.Name) and node.test.id == "TYPE_CHECKING":
            if self.ignore_type_checking_imports:
                return

        # assume other conditional imports represent real dependencies
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom):
        # For relative imports (level > 0), adjust the base module path
        if node.module is not None and node.level > 0:
            num_paths_to_strip = node.level - 1 if self.is_package else node.level
            base_path_parts = self.current_mod_path.split(".")
            if num_paths_to_strip:
                base_path_parts = base_path_parts[:-num_paths_to_strip]
            base_mod_path = ".".join(
                [*base_path_parts, node.module if node.module else ""]
            )
        else:
            base_mod_path = node.module or ""

        ignored_modules = self._get_ignored_modules(node.lineno)

        if ignored_modules is not None and len(ignored_modules) == 0:
            # Empty ignore list signifies blanket ignore of following import
            return

        for name_node in node.names:
            local_mod_path = f"{'.' * node.level}{node.module or ''}.{name_node.asname or name_node.name}"
            if ignored_modules is not None and (local_mod_path in ignored_modules):
                # This import is ignored by a tach-ignore directive
                continue

            global_mod_path = (
                f"{base_mod_path}.{name_node.name}" if node.module else name_node.name
            )
            if fs.is_project_import(self.project_root, global_mod_path):
                self.imports.append(
                    ProjectImport(
                        mod_path=global_mod_path,
                        line_number=node.lineno,
                    )
                )

    def visit_Import(self, node: ast.Import):
        ignored_modules = self._get_ignored_modules(node.lineno)
        if ignored_modules is not None and len(ignored_modules) == 0:
            # Empty ignore list signifies blanket ignore of following import
            return

        ignored_modules = ignored_modules or []
        for alias in node.names:
            if alias.name in ignored_modules or not fs.is_project_import(
                self.project_root, alias.name
            ):
                continue
            self.imports.append(
                ProjectImport(
                    mod_path=alias.name,
                    line_number=node.lineno,
                )
            )


def get_project_imports(
    project_root: str, file_path: str, ignore_type_checking_imports: bool = False
) -> list[ProjectImport]:
    file_content = fs.read_file(file_path)
    parsed_ast = fs.parse_ast(file_path)
    ignore_directives = get_ignore_directives(file_content)
    mod_path = fs.file_to_module_path(file_path)
    is_package = (
        file_path.endswith(os.path.sep + "__init__.py") or file_path == "__init__.py"
    )
    import_visitor = ImportVisitor(
        project_root=project_root,
        is_package=is_package,
        current_mod_path=mod_path,
        ignore_directives=ignore_directives,
        ignore_type_checking_imports=ignore_type_checking_imports,
    )
    import_visitor.visit(parsed_ast)
    return import_visitor.imports
