import ast
from typing import Optional, Union

from modguard import public
from modguard.core.public import PublicMember
from modguard.errors import ModguardParseError

from .utils import file_to_module_path


class ModguardImportVisitor(ast.NodeVisitor):
    def __init__(self, module_name: str):
        self.module_name = module_name
        self.import_found = False

    def visit_ImportFrom(self, node: ast.ImportFrom):
        if self.module_name:
            is_modguard_module_import = node.module is not None and (
                node.module == "modguard" or node.module.startswith("modguard.")
            )
            if is_modguard_module_import and any(
                alias.name == self.module_name for alias in node.names
            ):
                self.import_found = True
                return
        self.generic_visit(node)

    def visit_Import(self, node: ast.Import):
        for alias in node.names:
            if alias.name == "modguard":
                self.import_found = True
                return
        self.generic_visit(node)


def is_modguard_imported(parsed_ast: ast.AST, module_name: str = "") -> bool:
    modguard_import_visitor = ModguardImportVisitor(module_name)
    modguard_import_visitor.visit(parsed_ast)
    return modguard_import_visitor.import_found


class PublicMemberVisitor(ast.NodeVisitor):
    def __init__(self, current_mod_path: str, is_package: bool = False):
        self.is_modguard_public_imported = False
        self.current_mod_path = current_mod_path
        self.is_package = is_package
        self.public_members: list[PublicMember] = []

    def visit_ImportFrom(self, node: ast.ImportFrom):
        is_modguard_module_import = node.module is not None and (
            node.module == "modguard" or node.module.startswith("modguard.")
        )
        if is_modguard_module_import and any(
            alias.name == "public" for alias in node.names
        ):
            self.is_modguard_public_imported = True
        self.generic_visit(node)

    def visit_Import(self, node: ast.Import):
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_public_imported = True
        self.generic_visit(node)

    def _extract_allowlist(self, public_call: ast.Call) -> Optional[list[str]]:
        for kw in public_call.keywords:
            if kw.arg == "allowlist":
                allowlist_value = kw.value
                if isinstance(allowlist_value, ast.List):
                    return [
                        elt.value
                        for elt in allowlist_value.elts
                        if isinstance(elt, ast.Constant) and isinstance(elt.value, str)
                    ] or None
        return None

    def _add_public_member_from_decorator(
        self, node: Union[ast.FunctionDef, ast.ClassDef], decorator: ast.expr
    ):
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

    def visit_FunctionDef(self, node: ast.FunctionDef):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)
        self.generic_visit(node)

    def visit_ClassDef(self, node: ast.ClassDef):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call):
        parent_node = getattr(node, "parent")
        grandparent_node = getattr(parent_node, "parent")
        top_level = isinstance(parent_node, ast.Module)
        top_level_expr = isinstance(parent_node, ast.Expr) and isinstance(
            grandparent_node, ast.Module
        )
        is_raw_public_call = (
            isinstance(node.func, ast.Name) and node.func.id == "public"
        )
        is_modguard_public_call = (
            isinstance(node.func, ast.Attribute)
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == "modguard"
            and node.func.attr == "public"
        )
        if (
            self.is_modguard_public_imported
            and (top_level or top_level_expr)
            and (is_raw_public_call or is_modguard_public_call)
        ):
            # public() has been called at the top-level,
            if node.args:
                # if public is given positional arguments, add each as a public member
                self.public_members.extend(
                    (
                        PublicMember(
                            name=arg.id,
                            allowlist=self._extract_allowlist(node),
                        )
                        for arg in node.args
                        if isinstance(arg, ast.Name)
                    )
                )
            else:
                # if no positional arguments, we add a PublicMember for the whole module and return
                self.public_members = [
                    PublicMember(
                        name="",
                        allowlist=self._extract_allowlist(node),
                    )
                ]
            return
        self.generic_visit(node)

    def visit(self, node: ast.AST):
        # Inject a 'parent' attribute to each node for easier parent tracking
        for child in ast.iter_child_nodes(node):
            setattr(child, "parent", node)
        super().visit(node)


def get_public_members(file_path: str) -> list[PublicMember]:
    with open(file_path, "r") as file:
        file_content = file.read()

    try:
        parsed_ast = ast.parse(file_content)
    except SyntaxError as e:
        raise ModguardParseError(f"Syntax error in {file_path}: {e}")

    mod_path = file_to_module_path(file_path)
    public_member_visitor = PublicMemberVisitor(
        is_package=file_path.endswith("__init__.py"), current_mod_path=mod_path
    )
    public_member_visitor.visit(parsed_ast)
    return public_member_visitor.public_members


# TODO: handle re-exported members (follow imports?)
class MemberFinder(ast.NodeVisitor):
    def __init__(self, member_name: str):
        self.member_name = member_name
        # For functions and classes, matched_lineno is the start of the definition
        # because a decorator can be inserted directly before the definition
        # For assignments, matched_lineno is the end of the assignment
        # because a public(...) call can be inserted directly after the assignment
        self.matched_lineno: Optional[int] = None
        self.matched_assignment = False
        self.depth = 0

    def _check_assignment_target(
        self, target: Union[ast.expr, ast.Name, ast.Attribute, ast.Subscript]
    ):
        if isinstance(target, ast.Name) and target.id == self.member_name:
            self.matched_lineno = target.end_lineno
            self.matched_assignment = True
            return
        elif isinstance(target, ast.List) or isinstance(target, ast.Tuple):
            for elt in target.elts:
                if isinstance(elt, ast.Name) and elt.id == self.member_name:
                    self.matched_lineno = target.end_lineno
                    self.matched_assignment = True
                    return

    def visit_Assign(self, node: ast.Assign):
        if self.depth == 0:
            for target in node.targets:
                self._check_assignment_target(target)
        self.generic_visit(node)

    def visit_AnnAssign(self, node: ast.AnnAssign):
        if self.depth == 0:
            self._check_assignment_target(node.target)
        self.generic_visit(node)

    def visit_Global(self, node: ast.Global):
        if self.member_name in node.names:
            self.matched_lineno = node.end_lineno
            self.matched_assignment = True
            return
        self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef):
        if self.depth == 0 and node.name == self.member_name:
            self.matched_lineno = node.lineno
            return

        self.depth += 1
        self.generic_visit(node)
        self.depth -= 1

    def visit_ClassDef(self, node: ast.ClassDef):
        if self.depth == 0 and node.name == self.member_name:
            self.matched_lineno = node.lineno
            return

        self.depth += 1
        self.generic_visit(node)
        self.depth -= 1


def _public_module_prelude(should_import: bool = True) -> str:
    if should_import:
        return "import modguard\nmodguard.public()\n"
    return "modguard.public()\n"


IMPORT_MODGUARD = "import modguard"
PUBLIC_DECORATOR = "@modguard.public"
PUBLIC_CALL = "modguard.public"


@public
def mark_as_public(file_path: str, member_name: str = ""):
    with open(file_path, "r+") as file:
        file_content = file.read()
        file.seek(0)
        try:
            parsed_ast = ast.parse(file_content)
        except SyntaxError as e:
            raise ModguardParseError(f"Syntax error in {file_path}: {e}")
        modguard_public_is_imported = is_modguard_imported(parsed_ast, "public")
        if not member_name:
            file.write(
                _public_module_prelude(should_import=not modguard_public_is_imported)
                + file_content
            )
            return

        member_finder = MemberFinder(member_name)
        member_finder.visit(parsed_ast)
        if member_finder.matched_lineno is None:
            raise ModguardParseError(
                f"Failed to find member {member_name} in file {file_path}"
            )

        normal_lineno = member_finder.matched_lineno - 1
        file_lines = file_content.splitlines(keepends=True)
        if member_finder.matched_assignment:
            # Insert a call to public for the member after the assignment
            lines_to_write = [
                *file_lines[: normal_lineno + 1],
                f"{PUBLIC_CALL}({member_name})\n",
                *file_lines[normal_lineno + 1 :],
            ]
        else:
            # Insert a decorator before the function or class definition
            lines_to_write = [
                *file_lines[:normal_lineno],
                PUBLIC_DECORATOR + "\n",
                *file_lines[normal_lineno:],
            ]
        if not modguard_public_is_imported:
            lines_to_write = [IMPORT_MODGUARD + "\n", *lines_to_write]

        file.write("".join(lines_to_write))
