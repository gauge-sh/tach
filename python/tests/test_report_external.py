from __future__ import annotations

from unittest.mock import Mock

import pytest

from tach.extension import ProjectConfig
from tach.report import external_dependency_report


@pytest.fixture
def project_config():
    p = ProjectConfig()
    p.source_roots = [
        "src/pack-a/src",
        "src/pack-b/src",
        "src/pack-c/src",
        "src/pack-d/src",
        "src/pack-e/src",
        "src/pack-f/src",
        "src/pack-g/src",
    ]
    p.ignore_type_checking_imports = True
    return p


@pytest.fixture
def module_mapping(mocker):
    mock = Mock(return_value={"git": ["gitpython"]})
    mocker.patch("tach.utils.external.get_module_mappings", mock)


def test_report_multi_package_example(example_dir, project_config, module_mapping):
    project_root = example_dir / "multi_package"
    result = external_dependency_report(
        project_root=project_root,
        project_config=project_config,
        path=project_root / "src/pack-a/src/myorg/pack_a/__init__.py",
    )
    assert "gitpython" in result


def test_report_empty_multi_package_example(
    example_dir, project_config, module_mapping
):
    project_root = example_dir / "multi_package"
    result = external_dependency_report(
        project_root=project_root,
        project_config=project_config,
        path=project_root / "src/pack-b/src/myorg/pack_b/__init__.py",
    )
    assert "No external dependencies" in result


def test_report_raw_multi_package_example(example_dir, project_config, module_mapping):
    project_root = example_dir / "multi_package"
    result = external_dependency_report(
        project_root=project_root,
        project_config=project_config,
        path=project_root / "src/pack-a/src/myorg/pack_a/__init__.py",
        raw=True,
    )
    assert result == "gitpython"


def test_report_empty_raw_multi_package_example(
    example_dir, project_config, module_mapping
):
    project_root = example_dir / "multi_package"
    result = external_dependency_report(
        project_root=project_root,
        project_config=project_config,
        path=project_root / "src/pack-b/src/myorg/pack_b/__init__.py",
        raw=True,
    )
    assert result == ""
