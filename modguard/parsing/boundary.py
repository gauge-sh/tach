import ast
from typing import Optional

from modguard.core.boundary import BoundaryTrie
from modguard.public import public
from modguard import filesystem as fs
from .public import get_public_members


class BoundaryFinder(ast.NodeVisitor):
    def __init__(self):
        self.is_modguard_boundary_imported = False
        self.found_boundary = False

    def visit_ImportFrom(self, node: ast.ImportFrom):
        # Check if 'Boundary' is imported specifically from a 'modguard'-rooted module
        is_modguard_module_import = node.module is not None and (
            node.module == "modguard" or node.module.startswith("modguard.")
        )
        if is_modguard_module_import and any(
            alias.name == "Boundary" for alias in node.names
        ):
            self.is_modguard_boundary_imported = True

    def visit_Import(self, node: ast.Import):
        # Check if 'modguard' is imported
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_boundary_imported = True

    def visit_Call(self, node: ast.Call):
        if self.is_modguard_boundary_imported:
            if isinstance(node.func, ast.Attribute) and node.func.attr == "Boundary":
                if (
                    isinstance(node.func.value, ast.Name)
                    and node.func.value.id == "modguard"
                ):
                    self.found_boundary = True
            elif isinstance(node.func, ast.Name) and node.func.id == "Boundary":
                # This handles the case where 'Boundary' is imported directly: from modguard import Boundary
                # We are currently ignoring the case where this is still the wrong Boundary (if it has been re-assigned)
                self.found_boundary = True


@public
def has_boundary(file_path: str) -> bool:
    parsed_ast = fs.parse_ast(file_path)
    boundary_finder = BoundaryFinder()
    boundary_finder.visit(parsed_ast)
    return boundary_finder.found_boundary


BOUNDARY_PRELUDE = "import modguard\nmodguard.Boundary()\n"


@public
def add_boundary(file_path: str) -> None:
    file_content = fs.read_file(file_path)
    fs.write_file(file_path, BOUNDARY_PRELUDE + file_content)


@public
def build_boundary_trie(
    root: str, exclude_paths: Optional[list[str]] = None
) -> BoundaryTrie:
    boundary_trie = BoundaryTrie()
    # Add an 'outer boundary' containing the entire root path
    # This means a project will pass 'check' by default
    boundary_trie.insert(fs.file_to_module_path(root))

    for file_path in fs.walk_pyfiles(root, exclude_paths=exclude_paths):
        if has_boundary(file_path):
            mod_path = fs.file_to_module_path(file_path)
            boundary_trie.insert(mod_path)

    for file_path in fs.walk_pyfiles(root, exclude_paths=exclude_paths):
        mod_path = fs.file_to_module_path(file_path)
        public_members = get_public_members(file_path)
        for public_member in public_members:
            boundary_trie.register_public_member(mod_path, public_member)

    return boundary_trie
