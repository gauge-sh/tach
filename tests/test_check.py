from __future__ import annotations

import pytest

from tach.check import check_import
from tach.core import (
    PackageConfig,
    PackageNode,
    PackageTrie,
    ProjectConfig,
    TagDependencyRules,
)


@pytest.fixture
def test_config() -> PackageConfig:
    return PackageConfig(tags=["test"], strict=False)


@pytest.fixture
def project_config() -> ProjectConfig:
    return ProjectConfig(
        constraints=[
            TagDependencyRules(
                tag="domain_one", depends_on=["domain_one", "domain_three"]
            ),
            TagDependencyRules(tag="domain_two", depends_on=["domain_one"]),
            TagDependencyRules(tag="domain_three", depends_on=[]),
        ]
    )


@pytest.fixture
def package_trie() -> PackageTrie:
    return PackageTrie(
        root=PackageNode(
            is_end_of_path=False,
            full_path="",
            config=None,
            children={
                "domain_one": PackageNode(
                    is_end_of_path=True,
                    full_path="domain_one",
                    config=PackageConfig(tags=["domain_one"], strict=True),
                    interface_members=["public_fn"],
                    children={
                        "subdomain": PackageNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=PackageConfig(tags=["domain_one"], strict=True),
                            children={},
                        )
                    },
                ),
                "domain_two": PackageNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=PackageConfig(tags=["domain_two"], strict=False),
                    children={
                        "subdomain": PackageNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=PackageConfig(tags=["domain_two"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_three": PackageNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=PackageConfig(tags=["domain_three"], strict=False),
                    children={},
                ),
            },
        )
    )


@pytest.mark.parametrize(
    "file_mod_path,import_mod_path,expected_result",
    [
        ("domain_one", "domain_one", True),
        ("domain_one", "domain_one.subdomain", True),
        ("domain_one", "domain_one.core", True),
        ("domain_one", "domain_three", True),
        ("domain_two", "domain_one", True),
        ("domain_two", "domain_one.public_fn", True),
        ("domain_two.subdomain", "domain_one", True),
        ("domain_two", "external", True),
        ("external", "external", True),
        ("domain_two", "domain_one.private_fn", False),
        ("domain_three", "domain_one", False),
        ("domain_two", "domain_one.core", False),
        ("domain_two.subdomain", "domain_one.core", False),
        ("domain_two", "domain_three", False),
        ("domain_two", "domain_two.subdomain", False),
        ("external", "domain_three", False),
    ],
)
def test_check_import(
    project_config, package_trie, file_mod_path, import_mod_path, expected_result
):
    check_error = check_import(
        project_config=project_config,
        package_trie=package_trie,
        file_mod_path=file_mod_path,
        import_mod_path=import_mod_path,
    )
    result = check_error is None
    assert result == expected_result
