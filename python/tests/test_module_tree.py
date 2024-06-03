from __future__ import annotations

import pytest

from tach.core import ModuleConfig, ModuleNode, ModuleTree


@pytest.fixture
def test_config() -> ModuleConfig:
    return ModuleConfig(path="test", strict=False)


@pytest.fixture
def module_tree() -> ModuleTree:
    return ModuleTree(
        root=ModuleNode(
            is_end_of_path=True,
            full_path=".",
            config=ModuleConfig(path="<root>"),
            children={
                "domain_one": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_one",
                    config=ModuleConfig(path="test", strict=False),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=ModuleConfig(path="test", strict=False),
                            children={},
                        )
                    },
                ),
                "domain_two": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=ModuleConfig(path="test", strict=False),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=ModuleConfig(path="test", strict=False),
                            children={},
                        )
                    },
                ),
                "domain_three": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=ModuleConfig(path="test", strict=False),
                    children={},
                ),
            },
        )
    )


def test_iterate_over_empty_tree():
    tree = ModuleTree()
    assert list(tree) == [tree.root]


def test_iterate_over_populated_tree(module_tree):
    assert set(node.full_path for node in module_tree) == {
        ".",
        "domain_one",
        "domain_one.subdomain",
        "domain_two",
        "domain_two.subdomain",
        "domain_three",
    }


def test_get_nonexistent_path(module_tree):
    assert module_tree.get("fakepath") is None


def test_get_empty_path():
    tree = ModuleTree()
    assert tree.get("") is None


def test_get_actual_path(module_tree):
    assert module_tree.get("domain_one") is not None


def test_insert_empty_path(test_config):
    tree = ModuleTree()
    with pytest.raises(ValueError):
        tree.insert(test_config, "", [])


def test_insert_single_level_path(test_config):
    tree = ModuleTree()
    tree.insert(test_config, "domain", [])
    assert set(node.full_path for node in tree) == {".", "domain"}


def test_insert_multi_level_path(test_config):
    tree = ModuleTree()
    tree.insert(test_config, "domain.subdomain", [])
    assert set(node.full_path for node in tree) == {".", "domain.subdomain"}


def test_find_nearest_at_root(module_tree):
    # ModuleTree is not responsible for filtering to project imports
    module = module_tree.find_nearest("other_domain")
    assert module is module_tree.root


def test_find_nearest_in_single_domain(module_tree):
    module = module_tree.find_nearest("domain_one.thing")
    assert module.full_path == "domain_one"


def test_find_nearest_in_nested_domain(module_tree):
    module = module_tree.find_nearest("domain_two.subdomain.thing")
    assert module.full_path == "domain_two.subdomain"
