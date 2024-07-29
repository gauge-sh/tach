from __future__ import annotations

import pytest

from tach.cli import tach_sync


def test_valid_example_dir(example_dir, capfd):
    project_root = example_dir / "valid"
    with pytest.raises(SystemExit) as exc_info:
        # TODO not a great test, can change the example fs
        tach_sync(project_root=project_root)
    assert exc_info.value.code == 0
    captured = capfd.readouterr()
    assert "✅" in captured.out  # success state
