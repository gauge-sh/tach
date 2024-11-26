from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pytest

from tach.constants import DEFAULT_EXCLUDE_PATHS
from tach.extension import get_project_imports, set_excluded_paths


# Utility function to create temporary files with content
def create_temp_file(directory, filename, content):
    filepath = os.path.join(directory, filename)
    with open(filepath, "w") as f:
        f.write(content)
    return filepath


@pytest.fixture
def temp_project():
    with tempfile.TemporaryDirectory() as project_root:
        project_root = Path(project_root)
        # This is tech debt!!
        set_excluded_paths(
            str(project_root), DEFAULT_EXCLUDE_PATHS, use_regex_matching=True
        )

        # Creating some sample Python files in a nested structure
        (project_root / "a" / "b").mkdir(parents=True, exist_ok=True)
        (project_root / "d" / "e").mkdir(parents=True, exist_ok=True)
        (project_root / "local" / "g" / "h").mkdir(parents=True, exist_ok=True)
        (project_root / "local" / "m" / "n").mkdir(parents=True, exist_ok=True)
        (project_root / "parent").mkdir(parents=True, exist_ok=True)

        # Create __init__.py files in each directory
        (project_root / "a" / "__init__.py").touch()
        (project_root / "a" / "b" / "__init__.py").touch()
        (project_root / "d" / "__init__.py").touch()
        (project_root / "d" / "e" / "__init__.py").touch()
        (project_root / "local" / "__init__.py").touch()
        (project_root / "local" / "g" / "__init__.py").touch()
        (project_root / "local" / "g" / "h" / "__init__.py").touch()
        (project_root / "local" / "m" / "__init__.py").touch()
        (project_root / "local" / "m" / "n" / "__init__.py").touch()
        (project_root / "parent" / "__init__.py").touch()

        file1_content = """
import os
from local.file2 import b
"""
        file2_content = """
from ..file1 import y
"""
        file3_content = """
if TYPE_CHECKING:
    from local.file2 import c
"""
        file4_content = """
# tach-ignore(external dependency)
from a.b import c
from d.e import f  # tach-ignore(legacy import) f
from local.g.h import i, j  # tach-ignore(deprecated, using j instead) i

# tach-ignore(temporary workaround) k
from local.m.n import k, l

import file3
"""

        create_temp_file(project_root, "file1.py", file1_content)
        create_temp_file(project_root, "local/file2.py", file2_content)
        create_temp_file(project_root, "file3.py", file3_content)
        create_temp_file(project_root, "file4.py", file4_content)

        yield project_root


def test_regular_imports(temp_project):
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file1.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [("local.file2.b", 3)]
    assert result == expected


def test_relative_imports(temp_project):
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "local/file2.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [("file1.y", 2)]
    assert result == expected


def test_ignore_type_checking_imports(temp_project):
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file3.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = []
    assert result == expected


def test_include_type_checking_imports(temp_project):
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file3.py"),
        ignore_type_checking_imports=False,
        include_string_imports=False,
    )
    expected = [("local.file2.c", 3)]
    assert result == expected


def test_mixed_imports(temp_project):
    mixed_content = """
import sys
if TYPE_CHECKING:
    from .file2 import c
from ..file1 import x
"""
    create_temp_file(temp_project, "local/file4.py", mixed_content)
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "local/file4.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [("file1.x", 5)]
    assert result == expected

    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "local/file4.py"),
        ignore_type_checking_imports=False,
        include_string_imports=False,
    )
    expected = [("local.file2.c", 4), ("file1.x", 5)]
    assert result == expected


def test_external_imports(temp_project):
    external_content = """
import os
from external_module import something
"""
    create_temp_file(temp_project, "file5.py", external_content)
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file5.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = []  # 'os' and 'external_module' are not within the project root
    assert result == expected


def test_external_and_internal_imports(temp_project):
    mixed_content = """
import os
from file1 import c
from external_module import something
"""
    create_temp_file(temp_project, "file6.py", mixed_content)
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file6.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [
        ("file1.c", 3),
    ]
    assert result == expected


def test_ignored_imports(temp_project):
    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "file4.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [
        ("local.g.h.j", 5),  # only 'i' is ignored, 'j' is included
        ("local.m.n.l", 8),  # only 'k' is ignored, 'l' is included
        ("file3", 10),
    ]
    assert result == expected


def test_file_outside_source_root(temp_project, tmp_path):
    mixed_content = """
import os
from file1 import c
from external_module import something
"""

    path_outside_source_root = tmp_path / "outside_src_root.py"
    path_outside_source_root.write_text(mixed_content)

    result = get_project_imports(
        [str(temp_project)],
        str(path_outside_source_root),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [
        ("file1.c", 3),
    ]
    assert result == expected


def test_relative_import_from_parent(temp_project):
    # Create a nested directory structure
    (temp_project / "parent" / "child").mkdir(parents=True, exist_ok=True)

    # Create a file in the parent directory
    parent_file_content = """
def parent_function():
    pass
"""
    create_temp_file(temp_project / "parent", "parent_module.py", parent_file_content)

    # Create a file in the child directory with a relative import from the parent
    child_file_content = """
from .. import parent_module

def child_function():
    parent_module.parent_function()
"""
    create_temp_file(
        temp_project / "parent" / "child", "child_module.py", child_file_content
    )

    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "parent" / "child" / "child_module.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [("parent.parent_module", 2)]
    assert result == expected


def test_ignore_comments(temp_project):
    """Test different variations of tach-ignore comments"""
    content = """
# tach-ignore(skip all imports on next line)
from a.b import c, d

from d.e import f  # tach-ignore(deprecated) f
from local.g.h import i, j  # tach-ignore(using j instead) i

# tach-ignore(temporary) k
from local.m.n import k, l
"""
    create_temp_file(temp_project, "ignore_test.py", content)

    result = get_project_imports(
        [str(temp_project)],
        str(temp_project / "ignore_test.py"),
        ignore_type_checking_imports=True,
        include_string_imports=False,
    )
    expected = [
        ("local.g.h.j", 6),  # only 'i' is ignored
        ("local.m.n.l", 9),  # only 'k' is ignored
    ]
    assert result == expected
