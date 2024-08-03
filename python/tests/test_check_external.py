from __future__ import annotations

from pathlib import Path

import pytest

from tach.core.config import ProjectConfig
from tach.extension import check_external_dependencies


@pytest.fixture
def project_config():
    return ProjectConfig(
        source_roots=[
            "src/pack-a/src",
            "src/pack-b/src",
            "src/pack-c/src",
            "src/pack-d/src",
            "src/pack-e/src",
            "src/pack-f/src",
            "src/pack-g/src",
        ],
        ignore_type_checking_imports=True,
    )


@pytest.fixture
def module_mapping():
    return {
        "git": ["gitpython"],
    }


def test_check_external_dependencies_multi_package_example(
    example_dir, project_config, module_mapping
):
    project_root = example_dir / "multi_package"
    result = check_external_dependencies(
        project_root=str(project_root),
        source_roots=list(map(str, project_config.source_roots)),
        module_mappings=module_mapping,
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )
    # Assert no undeclared dependencies
    assert not result


def test_check_external_dependencies_invalid_multi_package_example(
    example_dir, project_config
):
    project_root = example_dir / "multi_package"
    result = check_external_dependencies(
        project_root=str(project_root),
        source_roots=list(map(str, project_config.source_roots)),
        module_mappings={},
        ignore_type_checking_imports=project_config.ignore_type_checking_imports,
    )
    expected_failure_path = Path(
        "src", "pack-a", "src", "myorg", "pack_a", "__init__.py"
    )
    assert set(result.keys()) == {str(expected_failure_path)}
    assert set(result[str(expected_failure_path)]) == {"git"}
