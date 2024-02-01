import ast
import os
from dataclasses import dataclass

from .boundary import BoundaryTrie
from .errors import ModguardParseError


@dataclass
class ErrorInfo:
    location: str
    error: str


def file_to_module_path(root_path: str, file_path: str):
    # Normalize the root path
    root_path = os.path.abspath(root_path)

    # Ensure the file path is absolute
    file_path = os.path.abspath(file_path)

    # Compute the relative path from root to file
    relative_path = os.path.relpath(file_path, root_path)

    # Replace os-specific path separators with '.'
    module_path = relative_path.replace(os.sep, ".")

    # Strip the extension and handle '__init__.py' files
    if module_path.endswith(".py"):
        module_path = module_path[:-3]  # Remove '.py' extension
    if module_path.endswith(".__init__"):
        module_path = module_path[:-9]  # Remove '.__init__' for package directories

    return module_path


class BoundaryFinder(ast.NodeVisitor):
    def __init__(self):
        self.is_modguard_boundary_imported = False

    def visit_ImportFrom(self, node):
        # Check if 'Boundary' is imported specifically from 'modguard'
        if node.module == "modguard" and any(
            alias.name == "Boundary" for alias in node.names
        ):
            self.is_modguard_boundary_imported = True
        self.generic_visit(node)

    def visit_Import(self, node):
        # Check if 'modguard' is imported, and if so, we will need additional checks when 'Boundary' is called
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_boundary_imported = True
        self.generic_visit(node)

    def visit_Call(self, node):
        if self.is_modguard_boundary_imported:
            if isinstance(node.func, ast.Attribute) and node.func.attr == "Boundary":
                if (
                    isinstance(node.func.value, ast.Name)
                    and node.func.value.id == "modguard"
                ):
                    return True
            elif isinstance(node.func, ast.Name) and node.func.id == "Boundary":
                # This handles the case where 'Boundary' is imported directly: from modguard import Boundary
                # We are currently ignoring the case where this is still the wrong Boundary (if it has been re-assigned)
                return True
        self.generic_visit(node)


def has_boundary(file_path: str) -> bool:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        boundary_finder = BoundaryFinder()
        return boundary_finder.visit(parsed_ast)
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")


class ImportVisitor(ast.NodeVisitor):
    def __init__(self, root_path: str, current_mod_path: str):
        self.root_path = root_path
        self.current_mod_path = current_mod_path
        self.imports = []

    def visit_ImportFrom(self, node):
        # For relative imports (level > 0), adjust the base module path
        if node.level > 0:
            base_path_parts = self.current_mod_path.split(".")[: -node.level]
            base_mod_path = ".".join(base_path_parts)
        else:
            base_mod_path = self.root_path  # Use root for absolute imports

        # Construct the full module path for the import
        if node.module:
            full_mod_path = (
                f"{base_mod_path}.{node.module}" if base_mod_path else node.module
            )
        else:
            full_mod_path = base_mod_path

        # Normalize the module path
        full_mod_path = full_mod_path.strip(".").replace("..", ".")

        self.imports.append(full_mod_path)

        # Continue traversing the tree
        self.generic_visit(node)


def get_imports(root: str, file_path: str) -> list[str]:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        mod_path = file_to_module_path(root, file_path)
        import_visitor = ImportVisitor(root_path=root, current_mod_path=mod_path)
        import_visitor.visit(parsed_ast)
        return import_visitor.imports
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")


def check(root: str) -> list[ErrorInfo]:
    # start recursively AST parsing all python files from the root
    # look for a call to modguard.Boundary in each file
    # if it is present, determine the python module of the file relative to the root
    # and update the boundaries to include this module (smarter data structure? wait until later)
    # also add a root boundary by default
    # also look for any calls to the modguard.Public decorator
    # and for each functiondef annotated with this decorator, find the nearest matching boundary,
    # and add the fully qualified path to the function to the boundaries mapping
    # then, start again recursively AST parsing all python files from the root
    # for each file
    #   determine the nearest boundary
    #   for each import
    #     find the nearest boundary to the source member (or ignore if outside all boundaries)
    #       check if the boundaries match
    #       check if the imported member is public (from the boundaries mapping)
    #       if neither, add an error to list of ErrorInfo
    # return all errors
    # Check if the root path is a directory
    if not os.path.isdir(root):
        return [ErrorInfo(location="", error=f"The path {root} is not a directory.")]

    boundary_trie = BoundaryTrie()
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            if filename.endswith(".py"):
                file_path = os.path.join(dirpath, filename)
                if has_boundary(file_path):
                    mod_path = file_to_module_path(root, file_path)
                    boundary_trie.insert(mod_path)

    errors = []
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            if filename.endswith(".py"):
                file_path = os.path.join(dirpath, filename)
                current_mod_path = file_to_module_path(root, file_path)
                current_nearest_boundary = boundary_trie.find_nearest(current_mod_path)
                import_mod_paths = get_imports(root, file_path)
                for mod_path in import_mod_paths:
                    nearest_boundary = boundary_trie.find_nearest(mod_path)
                    if current_nearest_boundary.full_path != nearest_boundary.full_path:
                        errors.append(
                            ErrorInfo(
                                location=file_path,
                                error=f"Import {mod_path} in {file_path} is blocked by boundary {nearest_boundary.full_path}",
                            )
                        )

    return errors
