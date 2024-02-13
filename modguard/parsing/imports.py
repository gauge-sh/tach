import ast
import re
from typing import Optional
from dataclasses import dataclass, field

from modguard import filesystem as fs
from modguard.public import public


@dataclass
class IgnoreDirective:
    lineno: int
    modules: list[str] = field(default_factory=list)


MODGUARD_IGNORE_REGEX = re.compile(r"# *modguard-ignore(( [\w.]+)*)$")


def get_ignore_directives(file_content: str) -> dict[int, IgnoreDirective]:
    ignores: dict[int, IgnoreDirective] = {}
    lines = file_content.splitlines()
    for lineno, line in enumerate(lines):
        normal_lineno = lineno + 1
        match = MODGUARD_IGNORE_REGEX.match(line)
        if match:
            ignored_modules = match.group(1)
            if ignored_modules:
                ignores[normal_lineno] = IgnoreDirective(
                    lineno=lineno + 1, modules=ignored_modules.split()
                )
            else:
                ignores[normal_lineno] = IgnoreDirective(lineno=lineno + 1)
    return ignores


class ImportVisitor(ast.NodeVisitor):
    def __init__(
        self,
        current_mod_path: str,
        is_package: bool = False,
        ignore_directives: Optional[dict[int, IgnoreDirective]] = None,
    ):
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.ignored_imports = ignore_directives or {}
        self.imports: list[str] = []

    def _get_ignored_modules(self, lineno: int) -> Optional[list[str]]:
        # Check for ignore directive at the previous line or on the current line
        directive = self.ignored_imports.get(lineno - 1) or self.ignored_imports.get(
            lineno
        )
        return directive.modules if directive else None

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
            local_mod_path = (
                f"{'.' * node.level}{node.module}.{name_node.asname or name_node.name}"
                if node.module
                else f"{'.' * node.level}{name_node.asname or name_node.name}"
            )
            if ignored_modules is not None and (local_mod_path in ignored_modules):
                # This import is ignored by a modguard-ignore directive
                continue

            global_mod_path = (
                f"{base_mod_path}.{name_node.name}" if node.module else name_node.name
            )
            self.imports.append(global_mod_path)

    def visit_Import(self, node: ast.Import):
        ignored_modules = self._get_ignored_modules(node.lineno)
        if ignored_modules is not None and len(ignored_modules) == 0:
            # Empty ignore list signifies blanket ignore of following import
            return

        ignored_modules = ignored_modules or []
        self.imports.extend(
            (alias.name for alias in node.names if alias.name not in ignored_modules)
        )


@public
def get_imports(file_path: str) -> list[str]:
    file_content = fs.read_file(file_path)
    parsed_ast = fs.parse_ast(file_path)
    ignore_directives = get_ignore_directives(file_content)
    mod_path = fs.file_to_module_path(file_path)
    import_visitor = ImportVisitor(
        is_package=file_path.endswith("__init__.py"),
        current_mod_path=mod_path,
        ignore_directives=ignore_directives,
    )
    import_visitor.visit(parsed_ast)
    return import_visitor.imports
