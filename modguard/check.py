import ast
import os
from dataclasses import dataclass

from .boundary import BoundaryTrie
from .errors import ModguardParseError


@dataclass
class ErrorInfo:
    location: str
    message: str


def file_to_module_path(file_path: str):
    # Assuming that the file_path has been 'canonicalized' and does not traverse multiple directories
    file_path = file_path.lstrip("./")
    if file_path == ".":
        return ""

    module_path = file_path.replace(os.sep, ".")

    if module_path.endswith(".py"):
        module_path = module_path[:-3]
    if module_path.endswith(".__init__"):
        module_path = module_path[:-9]
    if module_path == "__init__":
        return ""

    return module_path


class BoundaryFinder(ast.NodeVisitor):
    def __init__(self):
        self.is_modguard_boundary_imported = False
        self.found_boundary = False

    def visit_ImportFrom(self, node):
        # Check if 'Boundary' is imported specifically from 'modguard'
        if node.module == "modguard" and any(
            alias.name == "Boundary" for alias in node.names
        ):
            self.is_modguard_boundary_imported = True
        self.generic_visit(node)

    def visit_Import(self, node):
        # Check if 'modguard' is imported
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
                    self.found_boundary = True
            elif isinstance(node.func, ast.Name) and node.func.id == "Boundary":
                # This handles the case where 'Boundary' is imported directly: from modguard import Boundary
                # We are currently ignoring the case where this is still the wrong Boundary (if it has been re-assigned)
                self.found_boundary = True
        self.generic_visit(node)


def has_boundary(file_path: str) -> bool:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        boundary_finder = BoundaryFinder()
        boundary_finder.visit(parsed_ast)
        return boundary_finder.found_boundary
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")


class ImportVisitor(ast.NodeVisitor):
    def __init__(self, current_mod_path: str, is_package: bool = False):
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.imports = []

    def visit_ImportFrom(self, node):
        # For relative imports (level > 0), adjust the base module path
        if node.level > 0:
            num_paths_to_strip = node.level - 1 if self.is_package else node.level
            base_path_parts = self.current_mod_path.split(".")
            if num_paths_to_strip:
                base_path_parts = base_path_parts[:-num_paths_to_strip]
            base_mod_path = ".".join([*base_path_parts, node.module])
        else:
            base_mod_path = node.module

        for name_node in node.names:
            self.imports.append(f"{base_mod_path}.{name_node.asname or name_node.name}")

        self.generic_visit(node)


def get_imports(file_path: str) -> list[str]:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        mod_path = file_to_module_path(file_path)
        import_visitor = ImportVisitor(
            is_package=file_path.endswith("__init__.py"), current_mod_path=mod_path
        )
        import_visitor.visit(parsed_ast)
        return import_visitor.imports
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")


class PublicMemberVisitor(ast.NodeVisitor):
    def __init__(self, current_mod_path: str, is_package: bool = False):
        self.is_modguard_public_imported = False
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.public_members = []

    def visit_ImportFrom(self, node):
        if node.module == "modguard" and any(
            alias.name == "public" for alias in node.names
        ):
            self.is_modguard_public_imported = True
        self.generic_visit(node)

    def visit_Import(self, node):
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_public_imported = True
        self.generic_visit(node)

    def visit_FunctionDef(self, node):
        for decorator in node.decorator_list:
            if isinstance(decorator, ast.Name) and decorator.id == "public":
                self.public_members.append(node.name)
            elif isinstance(decorator, ast.Attribute) and decorator.attr == "public":
                value = decorator.value
                if isinstance(value, ast.Name) and value.id == "modguard":
                    self.public_members.append(node.name)
        self.generic_visit(node)

    def visit_ClassDef(self, node):
        for decorator in node.decorator_list:
            if isinstance(decorator, ast.Name) and decorator.id == "public":
                self.public_members.append(node.name)
            elif isinstance(decorator, ast.Attribute) and decorator.attr == "public":
                value = decorator.value
                if isinstance(value, ast.Name) and value.id == "modguard":
                    self.public_members.append(node.name)
        self.generic_visit(node)


def get_public_members(file_path: str) -> list[str]:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        mod_path = file_to_module_path(file_path)
        public_member_visitor = PublicMemberVisitor(
            is_package=file_path.endswith("__init__.py"), current_mod_path=mod_path
        )
        public_member_visitor.visit(parsed_ast)
        return public_member_visitor.public_members
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")


def build_boundary_trie(root: str) -> BoundaryTrie:
    boundary_trie = BoundaryTrie()
    # Add an 'outer boundary' containing the entire root path
    # This means a project will pass 'check' by default
    boundary_trie.insert(file_to_module_path(root))

    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            if filename.endswith(".py"):
                file_path = os.path.join(dirpath, filename)
                if has_boundary(file_path):
                    mod_path = file_to_module_path(file_path)
                    boundary_trie.insert(mod_path)

    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            if filename.endswith(".py"):
                file_path = os.path.join(dirpath, filename)
                mod_path = file_to_module_path(file_path)
                public_members = get_public_members(file_path)
                for public_member in public_members:
                    boundary_trie.register_public_member(f"{mod_path}.{public_member}")

    return boundary_trie


def check(root: str, exclude_paths:list[str] = None) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [ErrorInfo(location="", message=f"The path {root} is not a directory.")]

    # This 'canonicalizes' the root path, resolving directory traversal
    root = os.path.relpath(os.path.realpath(root))
    boundary_trie = build_boundary_trie(root)

    errors = []
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            if filename.endswith(".py"):
                file_path = os.path.join(dirpath, filename)
                current_mod_path = file_to_module_path(file_path)
                current_nearest_boundary = boundary_trie.find_nearest(current_mod_path)
                assert (
                    current_nearest_boundary is not None
                ), f"Checking file ({file_path}) outside of boundaries!"
                import_mod_paths = get_imports(file_path)
                for mod_path in import_mod_paths:
                    nearest_boundary = boundary_trie.find_nearest(mod_path)
                    # An imported module is allowed only in the following cases:
                    # * The module is not contained by a boundary [generally 3rd party]
                    # * The module's boundary is a child of the current boundary
                    # * The module is exported as public by its boundary
                    if (
                        nearest_boundary is not None
                        and not current_nearest_boundary.full_path.startswith(
                            nearest_boundary.full_path
                        )
                        and mod_path not in nearest_boundary.public_members
                    ):
                        errors.append(
                            ErrorInfo(
                                location=file_path,
                                message=f"Import {mod_path} in {file_path} is blocked by boundary {nearest_boundary.full_path}",
                            )
                        )

    return errors
