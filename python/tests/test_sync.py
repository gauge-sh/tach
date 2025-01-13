from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

import pytest

from tach.cli import tach_sync
from tach.parsing.config import parse_project_config


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "valid"
        shutil.copytree(project_root, temp_project_root)

        project_config = parse_project_config(root=temp_project_root)
        assert project_config is not None

        with pytest.raises(SystemExit) as exc_info:
            tach_sync(
                project_root=temp_project_root,
                project_config=project_config,
                exclude_paths=project_config.exclude,
                add=True,
            )

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.out  # success state
