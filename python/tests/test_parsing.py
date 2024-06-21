from __future__ import annotations

from pathlib import Path

import pytest
from pydantic import ValidationError

from tach.check import check
from tach.core import ModuleConfig, ProjectConfig
from tach.filesystem import file_to_module_path
from tach.parsing import parse_project_config


@pytest.fixture
def example_dir() -> Path:
    current_dir = Path(__file__).parent
    return current_dir / "example"


def test_file_to_mod_path():
    assert file_to_module_path(Path("."), Path("__init__.py")) == ""
    assert (
        file_to_module_path(Path("."), Path("domain_one/__init__.py")) == "domain_one"
    )
    assert (
        file_to_module_path(Path("."), Path("domain_one/interface.py"))
        == "domain_one.interface"
    )


def test_parse_valid_project_config(example_dir):
    result = parse_project_config(example_dir / "valid")
    assert result == ProjectConfig(
        modules=[
            ModuleConfig(path="domain_one", depends_on=["domain_two"]),
            ModuleConfig(path="domain_two", depends_on=["domain_one"]),
            ModuleConfig(path="domain_three"),
        ],
        exclude=["domain_thr.*"],
    )


def test_run_valid_project_config(example_dir):
    project_root = example_dir / "valid"
    project_config = parse_project_config(project_root)
    results = check(
        project_root=project_root,
        project_config=project_config,
        exclude_paths=project_config.exclude,
    )
    assert results.errors == []


def test_invalid_project_config(example_dir):
    with pytest.raises(ValidationError):
        parse_project_config(example_dir / "invalid")


def test_empty_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid" / "empty")
