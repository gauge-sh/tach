import pytest
from modguard.check import check, file_to_module_path, has_boundary, get_imports


def test_file_to_mod_path():
    assert file_to_module_path("__init__.py") == ""
    assert file_to_module_path("a/__init__.py") == "a"
    assert file_to_module_path("a/other.py") == "a.other"


def test_has_boundary():
    assert has_boundary("dummy_dir/a/__init__.py")
    assert not has_boundary("dummy_dir/a/other.py")


def test_get_imports():
    assert get_imports("dummy_dir/a/other.py") == []
    assert get_imports("dummy_dir/a/__init__.py") == [
        "modguard.Boundary",
        "dummy_dir.a.other.a",
    ]
    assert get_imports("dummy_dir/__init__.py") == [
        "modguard.Boundary",
        "dummy_dir.a.other.a",
    ]


def test_check():
    check_results = check("dummy_dir")
    assert len(check_results) == 2, "\n".join(
        (result.error for result in check_results)
    )
    check_results = check(".")
    assert len(check_results) == 2, "\n".join(
        (result.error for result in check_results)
    )
