# TODO: move this test
from modguard.core.config import ProjectConfig, ScopeDependencyRules, ModuleConfig
from modguard.filesystem import file_to_module_path
from modguard.filesystem.module import parse_module_config
from modguard.filesystem.project import parse_project_config
from modguard.parsing.boundary import has_boundary
from modguard.parsing.imports import get_imports


def test_file_to_mod_path():
    assert file_to_module_path("__init__.py") == ""
    assert file_to_module_path("domain_one/__init__.py") == "domain_one"
    assert file_to_module_path("domain_one/interface.py") == "domain_one.interface"


def test_has_boundary():
    assert has_boundary("example/domain_one/__init__.py")
    assert not has_boundary("example/domain_one/interface.py")


def test_get_imports():
    assert get_imports("example/domain_one/interface.py") == ["modguard.public"]
    assert set(get_imports("example/domain_one/__init__.py")) == {
        "modguard.boundary.Boundary",
        "example.domain_one.interface.domain_one_interface",
        "example.domain_one.interface.domain_one_var",
    }
    assert set(get_imports("example/__init__.py")) == {
        "modguard",
        "example.domain_one.interface.domain_one_interface",
        "example.domain_three.api.PublicForDomainTwo",
        "example.domain_four",
        "example.domain_four.subsystem.private_subsystem_call",
        "example.domain_one.interface.domain_one_var",
        "example.domain_five.inner.private_fn",
        "example.domain_five.pub_fn",
    }


def test_parse_valid_project_config():
    result = parse_project_config("example/valid/")
    assert result == ProjectConfig(
        ignore=["domain_three"],
        tags={
            "one": ScopeDependencyRules(depends_on=["two"]),
            "two": ScopeDependencyRules(depends_on=["one"]),
            "shared": ScopeDependencyRules(),
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
