import os

import pytest
from pydantic import ValidationError

from modguard.check import check, ErrorInfo
from modguard.core.config import ProjectConfig, ScopeDependencyRules, ModuleConfig
from modguard.parsing.config import parse_project_config, parse_module_config
from modguard.filesystem import file_to_module_path
from modguard import filesystem as fs


def test_file_to_mod_path():
    assert file_to_module_path("__init__.py") == ""
    assert file_to_module_path("domain_one/__init__.py") == "domain_one"
    assert file_to_module_path("domain_one/interface.py") == "domain_one.interface"


def test_parse_valid_project_config():
    result = parse_project_config("example/valid/")
    assert result == ProjectConfig(
        exclude=["domain_three"],
        constraints={
            "one": ScopeDependencyRules(depends_on=["two"]),
            "two": ScopeDependencyRules(depends_on=["one"]),
            "shared": ScopeDependencyRules(depends_on=[]),
        },
    )


def test_parse_valid_strict_module_config():
    result = parse_module_config("example/valid/domain_one")
    assert result == ModuleConfig(strict=True, tags=["one"])


def test_parse_valid_multi_tag_module_config():
    result = parse_module_config("example/valid/domain_two")
    assert result == ModuleConfig(strict=False, tags=["two", "shared"])


def test_module_with_no_config():
    result = parse_module_config("example/valid/domain_three")
    assert result is None


def test_invalid_project_config():
    with pytest.raises(ValidationError):
        parse_project_config("example/invalid/")


def test_empty_project_config():
    with pytest.raises(ValueError):
        parse_project_config("example/invalid/empty")


def test_invalid_module_config():
    with pytest.raises(ValidationError):
        parse_module_config("example/invalid")


def test_empty_module_config():
    with pytest.raises(ValueError):
        parse_module_config("example/invalid")


def test_exclude_hidden_paths_fails():
    current_dir = os.getcwd()
    hidden_project = "./example/invalid/hidden/"
    fs.chdir(hidden_project)
    project_config = parse_project_config()
    assert project_config.exclude_hidden_paths is False
    results = check(
        ".",
        project_config,
        exclude_hidden_paths=project_config.exclude_hidden_paths,
    )
    assert len(results) == 1
    assert results[0] == ErrorInfo(
        location="hidden",
        import_mod_path="",
        source_tag="",
        allowed_tags=[],
        exception_message="Module 'unhidden' is in strict mode. Only imports from the root of"
        " this module are allowed. The import 'unhidden.secret.shhhh' (in 'hidden') is not included in __all__.",
    )

    project_config.exclude_hidden_paths = True
    assert check(".", project_config, exclude_hidden_paths=True) == []
    fs.chdir(current_dir)
