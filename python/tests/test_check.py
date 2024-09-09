from __future__ import annotations

from itertools import chain
from unittest.mock import patch

import pytest

from tach.cli import tach_check
from tach.extension import ModuleConfig
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
            [
                ModuleConfig(path=path, strict=False)
                for path in chain(valid_modules, invalid_modules)
            ],
        )
        assert set(mod.path for mod in result.valid_modules) == set(valid_modules)
        assert set(mod.path for mod in result.invalid_modules) == set(invalid_modules)


@patch("tach.filesystem.module_to_pyfile_or_dir_path")
def test_validate_project_modules_root_is_always_valid(tmp_path):
    result = validate_project_modules(
        [tmp_path / "src"], [ModuleConfig.new_root_config()]
    )
    assert (
        len(result.valid_modules) == 1
        and result.valid_modules[0] == ModuleConfig.new_root_config()
    )
    assert not result.invalid_modules


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
