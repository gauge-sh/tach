from __future__ import annotations

import pytest

from tach.check import check_import
from tach.core import (
    ModuleConfig,
    ModuleNode,
    ModuleTree,
)


@pytest.fixture
def test_config() -> ModuleConfig:
    return ModuleConfig(path="test", strict=False)


@pytest.fixture
def module_tree() -> ModuleTree:
    return ModuleTree(
        root=ModuleNode(
            is_end_of_path=False,
            full_path="",
            config=None,
            children={
                "domain_one": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_one",
                    config=ModuleConfig(
                        path="domain_one",
                        depends_on=["domain_one.subdomain", "domain_three"],
                        strict=True,
                    ),
                    interface_members=["public_fn"],
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=ModuleConfig(
                                path="domain_one.subdomain", strict=True
                            ),
                            children={},
                        )
                    },
                ),
                "domain_two": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=ModuleConfig(
                        path="domain_two", depends_on=["domain_one"], strict=False
                    ),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=ModuleConfig(
                                path="domain_two",
                                depends_on=["domain_one"],
                                strict=False,
                            ),
                            children={},
                        )
                    },
                ),
                "domain_three": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=ModuleConfig(path="domain_three", strict=False),
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
def test_check_import(module_tree, file_mod_path, import_mod_path, expected_result):
    check_error = check_import(
        module_tree=module_tree,
        file_mod_path=file_mod_path,
        import_mod_path=import_mod_path,
    )
    result = check_error is None
    assert result == expected_result
