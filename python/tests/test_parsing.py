from __future__ import annotations

from pathlib import Path, PosixPath
from typing import TYPE_CHECKING

import pytest
from pydantic import ValidationError
from tach.parsing import find_modules_with_cycles, parse_project_config


def test_invalid_project_config(example_dir):
    with pytest.raises(ValidationError):
        parse_project_config(example_dir / "invalid")


def test_empty_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid" / "empty")
