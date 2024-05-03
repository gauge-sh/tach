import pytest
from pydantic import ValidationError

from tach.core import PackageConfig, TagDependencyRules, ProjectConfig
from tach.parsing.config import (
    parse_project_config_yml,
    parse_package_config_yml,
)
from tach.filesystem import file_to_module_path


def test_file_to_mod_path():
    assert file_to_module_path("__init__.py") == ""
    assert file_to_module_path("domain_one/__init__.py") == "domain_one"
    assert file_to_module_path("domain_one/interface.py") == "domain_one.interface"


def test_parse_valid_project_config():
    result = parse_project_config_yml("example/valid/")
    assert result == ProjectConfig(
        constraints=[
            TagDependencyRules(tag="one", depends_on=["two"]),
            TagDependencyRules(tag="two", depends_on=["one"]),
            TagDependencyRules(tag="three", depends_on=[]),
        ],
        exclude=["domain_thr.*"],
        exclude_hidden_paths=True,
    )


def test_parse_valid_strict_package_config():
    result = parse_package_config_yml("example/valid/domain_one")
    assert result == PackageConfig(strict=True, tags=["one"])


def test_parse_valid_multi_tag_package_config():
    result = parse_package_config_yml("example/valid/domain_two")
    assert result == PackageConfig(strict=False, tags=["two", "shared"])


def test_package_with_no_config():
    result = parse_package_config_yml("example/")
    assert result is None


def test_invalid_project_config():
    with pytest.raises(ValidationError):
        parse_project_config_yml("example/invalid/")


def test_empty_project_config():
    with pytest.raises(ValueError):
        parse_project_config_yml("example/invalid/empty")


def test_invalid_package_config():
    with pytest.raises(ValidationError):
        parse_package_config_yml("example/invalid")


def test_empty_package_config():
    with pytest.raises(ValueError):
        parse_package_config_yml("example/invalid")
