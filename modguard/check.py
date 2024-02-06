import ast
import os
import re
from dataclasses import dataclass, field
from typing import Generator, Optional

from .boundary import BoundaryTrie
from .errors import ModguardParseError
from .visibility import PublicMember


@dataclass
class ErrorInfo:
    location: str = ""
    import_mod_path: str = ""
    boundary_path: str = ""
    exception_message: str = ""

    @property
    def message(self) -> str:
        if self.exception_message:
            return self.exception_message
        if not all((self.location, self.import_mod_path, self.boundary_path)):
            return f"Unexpected error: ({[self.location, self.import_mod_path, self.boundary_path]})"
        return f"Import '{self.import_mod_path}' in {self.location} is blocked by boundary '{self.boundary_path}'"


def canonical(file_path: str) -> str:
    return os.path.relpath(os.path.realpath(file_path))


def walk_pyfiles(
    root: str, exclude_paths: list[str] = None
) -> Generator[tuple[str, str], None, None]:
    for dirpath, _, filenames in os.walk(root):
        for filename in filenames:
            file_path = canonical(os.path.join(dirpath, filename))
            if exclude_paths is not None and any(
                file_path.startswith(exclude_path) for exclude_path in exclude_paths
            ):
                # Treat excluded paths as invisible
                continue
            if filename.endswith(".py"):
                yield dirpath, filename


def file_to_module_path(file_path: str) -> str:
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
        # Check if 'Boundary' is imported specifically from a 'modguard'-rooted module
        if (node.module == "modguard" or node.module.startswith("modguard.")) and any(
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


@dataclass
class IgnoreDirective:
    lineno: int
    modules: list[str] = field(default_factory=list)


MODGUARD_IGNORE_REGEX = re.compile(r"# *modguard-ignore(( [\w.]+)*)$")


def get_ignore_directives(file_content: str) -> dict[int, IgnoreDirective]:
    ignores = {}
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
        ignore_directives: dict[int, IgnoreDirective] = None,
    ):
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.ignored_imports = ignore_directives or {}
        self.imports = []

    def _get_ignored_modules(self, lineno: int) -> Optional[list[str]]:
        # Check for ignore directive at the previous line or on the current line
        directive = self.ignored_imports.get(lineno - 1) or self.ignored_imports.get(
            lineno
        )
        return directive.modules if directive else None

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

        ignored_modules = self._get_ignored_modules(node.lineno)

        if ignored_modules is not None and len(ignored_modules) == 0:
            # Empty ignore list signifies blanket ignore of following import
            return self.generic_visit(node)

        for name_node in node.names:
            if ignored_modules is not None and (
                f"{'.' * node.level}{node.module}.{name_node.asname or name_node.name}"
                in ignored_modules
            ):
                # This import is ignored by a modguard-ignore directive
                continue

            self.imports.append(f"{base_mod_path}.{name_node.asname or name_node.name}")

        self.generic_visit(node)

    def visit_Import(self, node):
        ignored_modules = self._get_ignored_modules(node.lineno)
        if ignored_modules is not None and len(ignored_modules) == 0:
            # Empty ignore list signifies blanket ignore of following import
            return self.generic_visit(node)

        ignored_modules = ignored_modules or []
        self.imports.extend(
            (alias.name for alias in node.names if alias.name not in ignored_modules)
        )
        self.generic_visit(node)


def get_imports(file_path: str) -> list[str]:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
        ignore_directives = get_ignore_directives(file_content)
        mod_path = file_to_module_path(file_path)
        import_visitor = ImportVisitor(
            is_package=file_path.endswith("__init__.py"),
            current_mod_path=mod_path,
            ignore_directives=ignore_directives,
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
        self.public_members: list[PublicMember] = []

    def visit_ImportFrom(self, node):
        if (node.module == "modguard" or node.module.startswith("modguard.")) and any(
            alias.name == "public" for alias in node.names
        ):
            self.is_modguard_public_imported = True
        self.generic_visit(node)

    def visit_Import(self, node):
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_public_imported = True
        self.generic_visit(node)

    def _extract_allowlist(self, public_call: ast.Call) -> list[str]:
        for kw in public_call.keywords:
            if kw.arg == "allowlist":
                allowlist_value = kw.value
                if isinstance(allowlist_value, ast.List):
                    return [
                        elt.value
                        for elt in allowlist_value.elts
                        if isinstance(elt, ast.Constant) and isinstance(elt.value, str)
                    ]
        return []

    def _add_public_member_from_decorator(self, node: ast.AST, decorator: ast.expr):
        if (
            isinstance(decorator, ast.Call)
            and isinstance(decorator.func, ast.Name)
            and decorator.func.id == "public"
        ):
            # This means @public is called with arguments
            self.public_members.append(
                PublicMember(
                    name=node.name, allowlist=self._extract_allowlist(decorator)
                )
            )
        elif isinstance(decorator, ast.Name) and decorator.id == "public":
            self.public_members.append(PublicMember(name=node.name))
        elif isinstance(decorator, ast.Attribute) and decorator.attr == "public":
            value = decorator.value
            if isinstance(value, ast.Name) and value.id == "modguard":
                self.public_members.append(PublicMember(name=node.name))

    def visit_FunctionDef(self, node):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)
        self.generic_visit(node)

    def visit_ClassDef(self, node):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)
        self.generic_visit(node)

    def visit_Call(self, node):
        parent_node = node.parent
        top_level = isinstance(parent_node, ast.Module)
        top_level_expr = isinstance(parent_node, ast.Expr) and isinstance(
            getattr(parent_node, "parent"), ast.Module
        )
        if (
            self.is_modguard_public_imported
            and (top_level or top_level_expr)
            and isinstance(node.func, ast.Name)
            and node.func.id == "public"
        ):
            # public() has been called at the top-level,
            # so we add it as the sole PublicMember and return
            self.public_members = [
                PublicMember(
                    name="",
                    allowlist=self._extract_allowlist(node),
                )
            ]
            return
        self.generic_visit(node)

    def visit(self, node):
        # Inject a 'parent' attribute to each node for easier parent tracking
        for child in ast.iter_child_nodes(node):
            child.parent = node
        super().visit(node)


def get_public_members(file_path: str) -> list[PublicMember]:
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


def build_boundary_trie(root: str, exclude_paths: list[str] = None) -> BoundaryTrie:
    boundary_trie = BoundaryTrie()
    # Add an 'outer boundary' containing the entire root path
    # This means a project will pass 'check' by default
    boundary_trie.insert(file_to_module_path(root))

    for dirpath, filename in walk_pyfiles(root, exclude_paths=exclude_paths):
        file_path = os.path.join(dirpath, filename)
        if has_boundary(file_path):
            mod_path = file_to_module_path(file_path)
            boundary_trie.insert(mod_path)

    for dirpath, filename in walk_pyfiles(root, exclude_paths=exclude_paths):
        file_path = os.path.join(dirpath, filename)
        mod_path = file_to_module_path(file_path)
        public_members = get_public_members(file_path)
        for public_member in public_members:
            boundary_trie.register_public_member(mod_path, public_member)

    return boundary_trie


def check(root: str, exclude_paths: list[str] = None) -> list[ErrorInfo]:
    if not os.path.isdir(root):
        return [ErrorInfo(exception_message=f"The path {root} is not a directory.")]

    # This 'canonicalizes' the path arguments, resolving directory traversal
    root = canonical(root)
    exclude_paths = list(map(canonical, exclude_paths)) if exclude_paths else None

    boundary_trie = build_boundary_trie(root, exclude_paths=exclude_paths)

    errors = []
    for dirpath, filename in walk_pyfiles(root, exclude_paths=exclude_paths):
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
            import_mod_has_boundary = nearest_boundary is not None

            # * The module's boundary is a child of the current boundary
            import_mod_is_child_of_current = (
                import_mod_has_boundary
                and current_nearest_boundary.full_path.startswith(
                    nearest_boundary.full_path
                )
            )

            # * The module is exported as public by its boundary and is allowed in the current path
            import_mod_public_member_definition = (
                next(
                    (
                        public_member
                        for public_member_name, public_member in nearest_boundary.public_members.items()
                        if mod_path.startswith(public_member_name)
                    ),
                    None,
                )
                if import_mod_has_boundary
                else None
            )
            import_mod_is_public_and_allowed = (
                import_mod_public_member_definition is not None
                and (
                    import_mod_public_member_definition.allowlist is None
                    or any(
                        (
                            current_mod_path.startswith(allowed_path)
                            for allowed_path in import_mod_public_member_definition.allowlist
                        )
                    )
                )
            )

            if (
                not import_mod_has_boundary
                or import_mod_is_child_of_current
                or import_mod_is_public_and_allowed
            ):
                # This import is OK
                continue

            errors.append(
                ErrorInfo(
                    import_mod_path=mod_path,
                    boundary_path=nearest_boundary.full_path,
                    location=file_path,
                )
            )

    return errors
