from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

import pytest

from tach.cli import tach_sync


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_project_root = Path(temp_dir) / "valid"
        shutil.copytree(project_root, temp_project_root)

        with pytest.raises(SystemExit) as exc_info:
            tach_sync(project_root=temp_project_root, add=True)

        assert exc_info.value.code == 0
        captured = capfd.readouterr()
        assert "✅" in captured.out  # success state
