import re
from typing import Optional

from modguard import filesystem as fs
from modguard.core.boundary import BoundaryTrie
from modguard.public import public
from .public import get_public_members


@public
def has_boundary(file_path: str) -> bool:
    file_content = fs.read_file(file_path)
    # import modguard; modguard.Boundary()
    if re.search(r"(^|\n)import\s+modguard($|\n)", file_content):
        return bool(re.search(r"(^|\n)modguard\.Boundary\(", file_content))
    # from modguard.boundary import Boundary; Boundary()
    if re.search(r"(^|\n)from\s+modguard\.boundary\s+import.*Boundary", file_content):
        return bool(re.search(r"(^|\n)Boundary\(", file_content))
    # from modguard import boundary; boundary.Boundary()
    if re.search(r"(^|\n)from\s+modguard\s+import.*boundary", file_content):
        return bool(re.search(r"(^|\n)boundary\.Boundary\(", file_content))
    # import modguard.boundary; modguard.boundary.Boundary()
    if re.search(r"(^|\n)import\s+modguard\.boundary($|\n)", file_content):
        return bool(re.search(r"(^|\n)modguard\.boundary\.Boundary\(", file_content))
    return False


BOUNDARY_PRELUDE = "import modguard\nmodguard.Boundary()\n"


@public
def add_boundary(file_path: str) -> None:
    file_content = fs.read_file(file_path)
    fs.write_file(file_path, BOUNDARY_PRELUDE + file_content)


@public
def build_boundary_trie(
    root: str,
    exclude_paths: Optional[list[str]] = None,
    pyfiles: Optional[list[str]] = None,
) -> BoundaryTrie:
    boundary_trie = BoundaryTrie()
    # Add an 'outer boundary' containing the entire root path
    # This means a project will pass 'check' by default
    boundary_trie.insert(fs.file_to_module_path(root))
    pyfiles = pyfiles or list(fs.walk_pyfiles(root, exclude_paths=exclude_paths))

    for file_path in pyfiles:
        if has_boundary(file_path):
            mod_path = fs.file_to_module_path(file_path)
            boundary_trie.insert(mod_path)

    for file_path in pyfiles:
        mod_path = fs.file_to_module_path(file_path)
        public_members = get_public_members(file_path)
        for public_member in public_members:
            boundary_trie.register_public_member(mod_path, public_member)

    return boundary_trie
