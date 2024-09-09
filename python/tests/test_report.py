from __future__ import annotations

import os
from pathlib import Path

import pytest

from tach.errors import TachError
from tach.extension import ProjectConfig
from tach.report import report


@pytest.fixture
def mock_project_config() -> ProjectConfig:
    return ProjectConfig()


@pytest.fixture
def mock_cwd(tmp_path):
    try:
        original_path = os.getcwd()
        os.chdir(tmp_path)
        yield tmp_path
    finally:
        os.chdir(original_path)


# The code assumes that the cwd is within the project root
# due to pre-condition checks


def test_valid_path(mock_project_config, mock_cwd):
    mock_path = mock_cwd / "test.py"
    mock_path.touch()
    result = report(
        project_root=mock_cwd,
        path=Path("test.py"),
        project_config=mock_project_config,
    )
    assert result


def test_valid_dir(mock_project_config, mock_cwd):
    mock_path = mock_cwd / "test"
    mock_path.mkdir()
    result = report(
        project_root=mock_cwd,
        path=Path("test"),
        project_config=mock_project_config,
    )
    assert result


def test_valid_dir_trailing_slash(mock_project_config, mock_cwd):
    mock_path = mock_cwd / "test"
    mock_path.mkdir()
    result = report(
        project_root=mock_cwd,
        path=Path("test/"),
        project_config=mock_project_config,
    )
    assert result


def test_invalid_root(mock_project_config, mock_cwd):
    with pytest.raises(TachError):
        report(
            project_root=Path("Invalid!!"),
            path=Path("."),
            project_config=mock_project_config,
        )


def test_invalid_path(mock_project_config, mock_cwd):
    with pytest.raises(TachError):
        report(
            project_root=mock_cwd,
            path=Path("Invalid!!"),
            project_config=mock_project_config,
        )
