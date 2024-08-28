from __future__ import annotations

from pathlib import Path, PosixPath

import pytest
from pydantic import ValidationError

from tach.constants import DEFAULT_EXCLUDE_PATHS, ROOT_MODULE_SENTINEL_TAG
from tach.core import Dependency, ModuleConfig, ProjectConfig
from tach.core.config import CacheConfig
from tach.filesystem import file_to_module_path
from tach.parsing import find_modules_with_cycles, parse_project_config


@pytest.fixture
def example_dir() -> Path:
    current_dir = Path(__file__).parent
    return current_dir / "example"


def test_file_to_mod_path():
    assert file_to_module_path((Path("."),), Path("__init__.py")) == "."
    assert (
        file_to_module_path((Path("."),), Path("domain_one", "__init__.py"))
        == "domain_one"
    )
    assert (
        file_to_module_path((Path("."),), Path("domain_one", "interface.py"))
        == "domain_one.interface"
    )
    assert (
        file_to_module_path((Path("source", "root"),), Path("source", "root", "domain"))
        == "domain"
    )
    assert (
        file_to_module_path(
            (Path("src1"), Path("src2")), Path("src1", "core", "lib", "cat.py")
        )
        == "core.lib.cat"
    )


def test_parse_valid_project_config(example_dir):
    result = parse_project_config(example_dir / "valid")
    assert result == ProjectConfig(
        modules=[
            ModuleConfig(
                path="domain_one",
                depends_on=[Dependency(path="domain_two", deprecated=True)],
                strict=False,
            ),
            ModuleConfig(path="domain_three", depends_on=[], strict=False),
            ModuleConfig(
                path="domain_two",
                depends_on=[Dependency(path="domain_three", deprecated=False)],
                strict=False,
            ),
            ModuleConfig(
                path=ROOT_MODULE_SENTINEL_TAG,
                depends_on=[Dependency(path="domain_one", deprecated=False)],
                strict=False,
            ),
        ],
        cache=CacheConfig(backend="disk", file_dependencies=[], env_dependencies=[]),
        exclude=[*DEFAULT_EXCLUDE_PATHS, "domain_four"],
        source_roots=[PosixPath(".")],
        exact=True,
        disable_logging=False,
        ignore_type_checking_imports=True,
        forbid_circular_dependencies=True,
        use_regex_matching=True,
    )


def test_invalid_project_config(example_dir):
    with pytest.raises(ValidationError):
        parse_project_config(example_dir / "invalid")


def test_empty_project_config(example_dir):
    with pytest.raises(ValueError):
        parse_project_config(example_dir / "invalid" / "empty")


def test_valid_circular_dependencies(example_dir):
    project_config = parse_project_config(example_dir / "valid")
    assert project_config
    modules = project_config.modules
    modules_with_cycles = find_modules_with_cycles(modules)
    assert modules_with_cycles == []


def test_cycles_circular_dependencies(example_dir):
    project_config = parse_project_config(example_dir / "cycles")
    assert project_config
    modules = project_config.modules
    module_paths = find_modules_with_cycles(modules)
    assert set(module_paths) == {"domain_one", "domain_two", "domain_three"}
