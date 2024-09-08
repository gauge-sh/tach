from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture
def example_dir() -> Path:
    current_dir = Path(__file__).parent
    return current_dir / "example"
