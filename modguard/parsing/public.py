import ast
import re
from dataclasses import dataclass
from typing import Optional, Union, Any

import modguard
from modguard import filesystem as fs
from modguard.core.public import PublicMember
from .ast_visitor import EarlyExitNodeVisitor


@dataclass
class ModguardImportInfo:
    is_import_from: bool = False
    module_name: str = ""


def get_modguard_import_info(
    file_content: str, module_name: str = ""
) -> Optional[ModguardImportInfo]:
    # absolute import of modguard
    if re.search(r"(^|\n)import\s+modguard($|\n)", file_content):
        return ModguardImportInfo()

    if not module_name:
        # If no module, only absolute import is valid
        return None

    # absolute import of modguard.<module_name>
    if re.search(rf"(^|\n)import\s+modguard\.{module_name}($|\n)", file_content):
        return ModguardImportInfo(is_import_from=True, module_name=module_name)
    # from modguard import <module_name>
    if re.search(
        rf"(^|\n)from\s+modguard(\.\w+)?\s+import\s+{module_name}($|\n)", file_content
    ):
        return ModguardImportInfo(is_import_from=True, module_name=module_name)
    # from modguard import (..., <module_name>)
    # this is a best-effort regex that does NOT match multi-line imports correctly
    if re.search(
        rf"(^|\n)from\s+modguard(\.\w+)?\s+import.*{module_name}($|\n)", file_content
    ):
        return ModguardImportInfo(is_import_from=True, module_name=module_name)


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

    def visit_Import(self, node: ast.Import):
        for alias in node.names:
            if alias.name == "modguard":
                self.is_modguard_public_imported = True

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

    def _add_public_member_from_decorator(
        self,
        node: Union[ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef],
        decorator: ast.expr,
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
        elif (
            isinstance(decorator, ast.Call)
            and isinstance(decorator.func, ast.Attribute)
            and isinstance(decorator.func.value, ast.Name)
            and decorator.func.value.id == "modguard"
            and decorator.func.attr == "public"
        ):
            # This means @modguard.public is called with arguments
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

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)

    def visit_ClassDef(self, node: ast.ClassDef):
        if self.is_modguard_public_imported:
            for decorator in node.decorator_list:
                self._add_public_member_from_decorator(node=node, decorator=decorator)

    def visit_Call(self, node: ast.Call):
        is_raw_public_call = (
            isinstance(node.func, ast.Name) and node.func.id == "public"
        )
        is_modguard_public_call = (
            isinstance(node.func, ast.Attribute)
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == "modguard"
            and node.func.attr == "public"
        )
        if self.is_modguard_public_imported and (
            is_raw_public_call or is_modguard_public_call
        ):
            # public() has been called at the top-level,
            if node.args:
                # if public is given positional arguments, add each as a public member
                self.public_members.extend(
                    (
                        PublicMember(
                            name=arg.id if isinstance(arg, ast.Name) else arg.value,
                            allowlist=self._extract_allowlist(node),
                        )
                        for arg in node.args
                        if isinstance(arg, ast.Name) or isinstance(arg, ast.Constant)
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


def get_public_members(file_path: str) -> list[PublicMember]:
    parsed_ast = fs.parse_ast(file_path)
    mod_path = fs.file_to_module_path(file_path)
    public_member_visitor = PublicMemberVisitor(
        is_package=file_path.endswith("__init__.py"), current_mod_path=mod_path
    )
    public_member_visitor.visit(parsed_ast)
    return public_member_visitor.public_members


class MemberFinder(EarlyExitNodeVisitor):
    def __init__(self, member_name: str, *args: list[Any], **kwargs: dict[Any, Any]):
        super().__init__(*args, **kwargs)
        self.member_name = member_name
        self.start_lineno: Optional[int] = None
        # For assignments, end_lineno is the end of the assignment value expression
        # because a public(...) call should be inserted directly after the assignment
        self.end_lineno: Optional[int] = None
        self.matched_assignment = False

    def _check_assignment(
        self,
        node: Union[ast.Assign, ast.AnnAssign],
        target: Union[ast.expr, ast.Name, ast.Attribute, ast.Subscript],
    ):
        if isinstance(target, ast.Name) and target.id == self.member_name:
            self.start_lineno = target.lineno
            self.end_lineno = node.end_lineno
            self.matched_assignment = True
            self.set_exit()
            return
        elif isinstance(target, ast.List) or isinstance(target, ast.Tuple):
            for elt in target.elts:
                if isinstance(elt, ast.Name) and elt.id == self.member_name:
                    self.start_lineno = target.lineno
                    self.matched_lineno = node.end_lineno
                    self.matched_assignment = True
                    self.set_exit()
                    return

    def visit_Assign(self, node: ast.Assign):
        for target in node.targets:
            self._check_assignment(node, target)

    def visit_AnnAssign(self, node: ast.AnnAssign):
        self._check_assignment(node, node.target)

    def visit_Global(self, node: ast.Global):
        if self.member_name in node.names:
            self.start_lineno = node.lineno
            self.matched_assignment = True
            self.set_exit()
            return

    def visit_FunctionDef(self, node: ast.FunctionDef):
        if node.name == self.member_name:
            self.start_lineno = node.lineno
            self.set_exit()
            return

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef):
        if node.name == self.member_name:
            self.start_lineno = node.lineno
            self.set_exit()
            return

    def visit_ClassDef(self, node: ast.ClassDef):
        if node.name == self.member_name:
            self.start_lineno = node.lineno
            self.set_exit()
            return


WHITESPACE_REGEX = re.compile(r"^((\s)*)")
IMPORT_MODGUARD = "import modguard"
MODGUARD_PUBLIC_DECORATOR = "@modguard.public"
PUBLIC_DECORATOR = "@public"
MODGUARD_PUBLIC = "modguard.public"
PUBLIC = "public"


@modguard.public
def mark_as_public(file_path: str, member_name: str = ""):
    file_content = fs.read_file(file_path)
    parsed_ast = fs.parse_ast(file_path)
    modguard_import_info = get_modguard_import_info(file_content, "public")
    public_fn_name = (
        PUBLIC
        if modguard_import_info and modguard_import_info.is_import_from
        else MODGUARD_PUBLIC
    )
    public_decorator_name = (
        PUBLIC_DECORATOR
        if modguard_import_info and modguard_import_info.is_import_from
        else MODGUARD_PUBLIC_DECORATOR
    )

    if not member_name or member_name == "*":
        fs.write_file(
            file_path,
            (f"{IMPORT_MODGUARD}\n" if modguard_import_info is None else "")
            + file_content
            + f"{public_fn_name}()\n",
        )
        return

    member_finder = MemberFinder(member_name)
    member_finder.visit(parsed_ast)

    file_lines = file_content.splitlines(keepends=True)
    lines_to_write: list[str]
    if member_finder.start_lineno is None:
        # The member name was not found, which probably means it is dynamic
        # Add a public call with the member name as a string
        lines_to_write = [
            *file_lines,
            f'{public_fn_name}("{member_name}")\n',
        ]
    else:
        starting_line = file_lines[member_finder.start_lineno - 1]
        starting_whitespace_match = WHITESPACE_REGEX.match(starting_line)
        assert (
            starting_whitespace_match
        ), f"Whitespace regex should always match.\n{starting_line}"

        # The member name was found
        if member_finder.matched_assignment:
            assert (
                member_finder.end_lineno is not None
            ), f"Expected to find end_lineno on matched assignment. [{file_path}, {member_name}]"

            # Insert a call to public for the member after the assignment
            lines_to_write = [
                *file_lines[: member_finder.end_lineno],
                f"{starting_whitespace_match.group(1) or ''}{public_fn_name}({member_name})\n",
                *file_lines[member_finder.end_lineno :],
            ]
        else:
            # Insert a decorator before the function or class definition
            lines_to_write = [
                *file_lines[: member_finder.start_lineno - 1],
                f"{starting_whitespace_match.group(1) or ''}{public_decorator_name}\n",
                *file_lines[member_finder.start_lineno - 1 :],
            ]
    if modguard_import_info is None:
        lines_to_write = [IMPORT_MODGUARD + "\n", *lines_to_write]

    fs.write_file(file_path, "".join(lines_to_write))
