# TODO: move this test
from modguard.filesystem import file_to_module_path
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
        "example.domain_five.inner.private",
        "example.domain_five.pub_fn",
    }
