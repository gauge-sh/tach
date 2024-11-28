from __future__ import annotations

from pathlib import Path
from unittest.mock import Mock

import pytest

from tach.extension import ProjectConfig
from tach.report import external_dependency_report


@pytest.fixture
def project_config():
    p = ProjectConfig()
    p.source_roots = [
        Path("src/pack-a/src"),
        Path("src/pack-b/src"),
        Path("src/pack-c/src"),
        Path("src/pack-d/src"),
        Path("src/pack-e/src"),
        Path("src/pack-f/src"),
        Path("src/pack-g/src"),
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
    assert [line for line in result.splitlines() if not line.startswith("#")] == [
        "gitpython"
    ]


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
