from __future__ import annotations

from itertools import chain
from unittest.mock import patch

import pytest

from tach.check import check_import
from tach.cli import tach_check
from tach.core import (
    ModuleConfig,
)
from tach.core.config import RootModuleConfig
from tach.filesystem import validate_project_modules


@pytest.mark.parametrize(
    "valid_modules,invalid_modules",
    [
        (["mod.a"], []),
        ([], ["mod.b"]),
        (["mod.a", "mod.b"], ["mod.c"]),
        (["mod.a", "mod.b"], ["mod.c", "mod.d"]),
    ],
)
def test_validate_project_modules(tmp_path, valid_modules, invalid_modules):
    def mock_fs_check(source_roots, module_path):
        return module_path in valid_modules

    mock_source_root = tmp_path / "src"
    with patch("tach.filesystem.module_to_pyfile_or_dir_path", wraps=mock_fs_check):
        result = validate_project_modules(
            [mock_source_root],
            [ModuleConfig(path=path) for path in chain(valid_modules, invalid_modules)],
        )
        assert set(mod.path for mod in result.valid_modules) == set(valid_modules)
        assert set(mod.path for mod in result.invalid_modules) == set(invalid_modules)


@patch("tach.filesystem.module_to_pyfile_or_dir_path")
def test_validate_project_modules_root_is_always_valid(tmp_path):
    result = validate_project_modules([tmp_path / "src"], [RootModuleConfig()])
    assert (
        len(result.valid_modules) == 1 and result.valid_modules[0] == RootModuleConfig()
    )
    assert not result.invalid_modules


@pytest.mark.parametrize(
    "file_mod_path,import_mod_path,expected_result",
    [
        ("domain_one", "domain_one", True),
        ("domain_one", "domain_one.core", True),
        ("domain_one", "domain_three", True),
        ("domain_two", "domain_one", True),
        ("domain_two", "domain_one.public_fn", True),
        ("domain_two.subdomain", "domain_one", True),
        ("domain_two", "external", True),
        ("external", "external", True),
        ("domain_two", "domain_one.private_fn", False),
        ("domain_three", "domain_one", False),
        ("domain_two", "domain_one.core", False),
        ("domain_two.subdomain", "domain_one.core", False),
        ("domain_two", "domain_three", False),
        ("domain_two", "domain_two.subdomain", False),
        ("external", "domain_three", False),
    ],
)
def test_check_import(module_tree, file_mod_path, import_mod_path, expected_result):
    check_error = check_import(
        module_tree=module_tree,
        file_mod_path=file_mod_path,
        import_mod_path=import_mod_path,
    )
    result = check_error is None
    assert result == expected_result


def test_check_deprecated_import(module_tree):
    check_error = check_import(
        module_tree=module_tree,
        file_mod_path="domain_one",
        import_mod_path="domain_one.subdomain",
    )
    assert check_error is not None
    assert check_error.is_deprecated


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"
    with pytest.raises(SystemExit) as exc_info:
        tach_check(project_root=project_root)
    assert exc_info.value.code == 0
    captured = capfd.readouterr()
    assert "✅" in captured.out  # success state
    assert "‼️" in captured.err  # deprecated warning


def test_valid_example_dir_monorepo(example_dir):
    project_root = example_dir / "monorepo"
    with pytest.raises(SystemExit) as exc_info:
        tach_check(project_root=project_root)
    assert exc_info.value.code == 0
