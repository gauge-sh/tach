from __future__ import annotations

import os

import pytest

from tach.core.config import ProjectConfig
from tach.errors import TachError
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
        root=str(mock_cwd), path=str(mock_path), project_config=mock_project_config
    )
    assert result


def test_invalid_root(mock_project_config, mock_cwd):
    with pytest.raises(TachError):
        report(root="Invalid!!", path=str(mock_cwd), project_config=mock_project_config)


def test_invalid_path(mock_project_config, mock_cwd):
    with pytest.raises(TachError):
        report(root=str(mock_cwd), path="Invalid!!", project_config=mock_project_config)
