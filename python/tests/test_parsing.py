from __future__ import annotations

from pathlib import Path

import pytest
from pydantic import ValidationError

from tach.constants import ROOT_MODULE_SENTINEL_TAG
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
        file_to_module_path(Path("."), Path("domain_one", "__init__.py"))
        == "domain_one"
    )
    assert (
        file_to_module_path(Path("."), Path("domain_one", "interface.py"))
        == "domain_one.interface"
    )
    assert (
        file_to_module_path(Path("source", "root"), Path("source", "root", "domain"))
        == "domain"
    )


def test_parse_valid_project_config(example_dir):
    result = parse_project_config(example_dir / "valid")
    assert result == ProjectConfig(
        modules=[
            ModuleConfig(path="domain_one", depends_on=["domain_two"]),
            ModuleConfig(path="domain_two", depends_on=["domain_one"]),
            ModuleConfig(path=ROOT_MODULE_SENTINEL_TAG, depends_on=["domain_one"]),
        ],
        exclude=["domain_thr.*"],
        exact=True,
    )


def test_invalid_project_config(example_dir):
    with pytest.raises(ValidationError):
        parse_project_config(example_dir / "invalid")


def test_empty_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid" / "empty")
