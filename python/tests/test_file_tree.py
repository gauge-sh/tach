from __future__ import annotations

import pytest

from tach.interactive.modules import FileNode, FileTree


@pytest.fixture
def project_root(tmp_path):
    (tmp_path / "dir1").mkdir()
    (tmp_path / "dir2").mkdir()
    (tmp_path / "dir1" / "__init__.py").touch()
    (tmp_path / "dir1" / "file1.py").touch()
    (tmp_path / "dir2" / "file2.py").touch()
    return tmp_path


def test_build_from_path(project_root):
    tree = FileTree.build_from_path(project_root)
    assert isinstance(tree, FileTree)
    assert tree.root.full_path == project_root
    assert tree.root.is_dir
    assert str(project_root / "dir1") in tree.nodes
    assert str(project_root / "dir2") in tree.nodes
    assert str(project_root / "dir1" / "file1.py") in tree.nodes
    assert str(project_root / "dir2" / "file2.py") in tree.nodes


def test_visible_children_property(project_root):
    node = FileNode.build_from_path(project_root / "dir1")
    node.expanded = True
    child_node = FileNode.build_from_path(project_root / "dir1" / "file1.py")
    node.children.append(child_node)
    assert node.visible_children == [child_node]
    node.expanded = False
    assert node.visible_children == []


def test_set_modules(project_root):
    tree = FileTree.build_from_path(project_root)
    tree.initialize_modules([project_root / "dir1" / "file1.py"])
    assert tree.nodes[str(project_root / "dir1" / "file1.py")].is_module


def test_set_source_root(project_root):
    tree = FileTree.build_from_path(project_root)
    new_source_root = tree.nodes[str(project_root / "dir2")]
    tree.initialize_source_roots([new_source_root.full_path])
    assert new_source_root.is_source_root


def test_siblings_method(project_root):
    tree = FileTree.build_from_path(project_root)
    node = tree.nodes[str(project_root / "dir1" / "file1.py")]
    siblings = node.siblings(include_self=True)
    assert node in siblings
    siblings = node.siblings(include_self=False)
    assert node not in siblings


def test_exclude_single_file(project_root):
    exclude_paths = [r"dir1/file1\.py"]
    tree = FileTree.build_from_path(
        project_root, exclude_paths=exclude_paths, use_regex_matching=True
    )
    assert str(project_root / "dir1" / "file1.py") not in tree.nodes
    assert str(project_root / "dir1") in tree.nodes


def test_exclude_entire_directory(project_root):
    exclude_paths = [r"dir2/"]
    tree = FileTree.build_from_path(
        project_root, exclude_paths=exclude_paths, use_regex_matching=True
    )
    assert str(project_root / "dir2") not in tree.nodes
    assert str(project_root / "dir1") in tree.nodes
    assert str(project_root / "dir2" / "file2.py") not in tree.nodes
    assert str(project_root / "dir2" / "nested_dir") not in tree.nodes


def test_exclude_nested_directory(project_root):
    exclude_paths = [r"dir2/nested_dir/"]
    tree = FileTree.build_from_path(
        project_root, exclude_paths=exclude_paths, use_regex_matching=True
    )
    assert str(project_root / "dir2") in tree.nodes
    assert str(project_root / "dir2" / "nested_dir") not in tree.nodes
    assert str(project_root / "dir2" / "nested_dir" / "file4.py") not in tree.nodes


def test_exclude_multiple_patterns(project_root):
    exclude_paths = [r"dir1/.*", r"dir2/nested_dir/"]
    tree = FileTree.build_from_path(
        project_root, exclude_paths=exclude_paths, use_regex_matching=True
    )
    assert str(project_root / "dir1") not in tree.nodes
    assert str(project_root / "dir2") in tree.nodes
    assert str(project_root / "dir2" / "nested_dir") not in tree.nodes
