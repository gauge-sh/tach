from __future__ import annotations

import os
from pathlib import Path

import pytest

from tach.errors import TachError
from tach.extension import ProjectConfig
from tach.parsing.config import parse_project_config
from tach.report import report


@pytest.fixture
def empty_config() -> ProjectConfig:
    return ProjectConfig()


@pytest.fixture
def tmp_project(tmp_path):
    """Create a temporary project directory structure"""
    project_dir = tmp_path / "project"
    project_dir.mkdir()
    original_cwd = os.getcwd()
    os.chdir(project_dir)
    yield project_dir
    os.chdir(original_cwd)


@pytest.fixture
def example_valid_dir(example_dir):
    original_cwd = os.getcwd()
    os.chdir(example_dir / "valid")
    yield example_dir / "valid"
    os.chdir(original_cwd)


def test_valid_file(empty_config, tmp_project):
    test_file = tmp_project / "test.py"
    test_file.touch()
    result = report(
        project_root=tmp_project,
        path=Path("test.py"),
        project_config=empty_config,
    )
    assert result


def test_valid_directory(empty_config, tmp_project):
    test_dir = tmp_project / "test"
    test_dir.mkdir()
    result = report(
        project_root=tmp_project,
        path=Path("test"),
        project_config=empty_config,
    )
    assert result


def test_valid_directory_trailing_slash(empty_config, tmp_project):
    test_dir = tmp_project / "test"
    test_dir.mkdir()
    result = report(
        project_root=tmp_project,
        path=Path("test/"),
        project_config=empty_config,
    )
    assert result


def test_invalid_project_root(empty_config, tmp_project):
    with pytest.raises(TachError):
        report(
            project_root=Path("Invalid!!"),
            path=Path("."),
            project_config=empty_config,
        )


def test_invalid_path(empty_config, tmp_project):
    with pytest.raises(TachError):
        report(
            project_root=tmp_project,
            path=Path("Invalid!!"),
            project_config=empty_config,
        )


def test_report_valid_domain_one(example_valid_dir):
    project_config = parse_project_config(example_valid_dir)
    result = report(
        project_root=example_valid_dir,
        path=Path("domain_one"),
        project_config=project_config,
    )

    dependencies, usages = result.split("Usages of 'domain_one'")
    assert "domain_two.x" in dependencies
    assert "domain_one.x" in usages


def test_report_valid_domain_two(example_valid_dir):
    project_config = parse_project_config(example_valid_dir)
    result = report(
        project_root=example_valid_dir,
        path=Path("domain_two"),
        project_config=project_config,
    )

    dependencies, usages = result.split("Usages of 'domain_two'")
    assert "domain_three.x" in dependencies
    assert "domain_two.x" in usages


def test_report_raw_output(example_valid_dir):
    project_config = parse_project_config(example_valid_dir)
    result = report(
        project_root=example_valid_dir,
        path=Path("domain_one"),
        project_config=project_config,
        raw=True,
    )
    assert result.strip() == (
        """# Module Dependencies
domain_two
# Module Usages
."""
    )
