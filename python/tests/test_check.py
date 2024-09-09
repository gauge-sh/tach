from __future__ import annotations

import pytest

from tach.cli import tach_check


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
