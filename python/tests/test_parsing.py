from __future__ import annotations

import pytest

from tach.parsing import parse_project_config


def test_invalid_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid")


def test_empty_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid" / "empty")
