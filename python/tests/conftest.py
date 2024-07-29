from __future__ import annotations

from pathlib import Path

import pytest

from tach.core import (
    Dependency,
    ModuleConfig,
    ModuleNode,
    ModuleTree,
)


@pytest.fixture
def example_dir() -> Path:
    current_dir = Path(__file__).parent
    return current_dir / "example"


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
                        depends_on=[
                            Dependency(path="domain_one.subdomain", deprecated=True),
                            Dependency(path="domain_three"),
                        ],
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
                        path="domain_two",
                        depends_on=[Dependency(path="domain_one")],
                        strict=False,
                    ),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=ModuleConfig(
                                path="domain_two",
                                depends_on=[Dependency(path="domain_one")],
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
