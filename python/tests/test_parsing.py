from __future__ import annotations

import os

import pytest
from pydantic import ValidationError

from tach import filesystem as fs
from tach.check import check
from tach.core import ModuleConfig, ProjectConfig
from tach.filesystem import file_to_module_path
from tach.parsing import parse_project_config


def test_file_to_mod_path():
    assert file_to_module_path("__init__.py") == ""
    assert file_to_module_path("domain_one/__init__.py") == "domain_one"
    assert file_to_module_path("domain_one/interface.py") == "domain_one.interface"


def test_parse_valid_project_config():
    result = parse_project_config("example/valid/")
    assert result == ProjectConfig(
        modules=[
            ModuleConfig(path="domain_one", depends_on=["domain_two"]),
            ModuleConfig(path="domain_two", depends_on=["domain_one"]),
            ModuleConfig(path="domain_three"),
        ],
        exclude=["domain_thr.*"],
    )


def test_run_valid_project_config():
    current_dir = os.getcwd()
    try:
        project = "./example/valid/"
        fs.chdir(project)
        project_config = parse_project_config()
        results = check(
            ".",
            project_config,
            exclude_paths=project_config.exclude,
        )
        assert results == []
    finally:
        # Make sure not to dirty the test directory state
        fs.chdir(current_dir)


def test_invalid_project_config():
    with pytest.raises(ValidationError):
        parse_project_config("example/invalid/")


def test_empty_project_config():
    with pytest.raises(ValueError):
        parse_project_config("example/invalid/empty")
