from __future__ import annotations

import pytest

from tach.core import PackageConfig, PackageNode, PackageTrie


@pytest.fixture
def test_config() -> PackageConfig:
    return PackageConfig(tags=["test"], strict=False)


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
                    config=PackageConfig(tags=["test"], strict=False),
                    children={
                        "subdomain": PackageNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=PackageConfig(tags=["test"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_two": PackageNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=PackageConfig(tags=["test"], strict=False),
                    children={
                        "subdomain": PackageNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=PackageConfig(tags=["test"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_three": PackageNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=PackageConfig(tags=["test"], strict=False),
                    children={},
                ),
            },
        )
    )


def test_iterate_over_empty_trie():
    assert list(PackageTrie()) == []


def test_iterate_over_populated_trie(package_trie):
    assert set((node.full_path for node in package_trie)) == {
        "domain_one",
        "domain_one.subdomain",
        "domain_two",
        "domain_two.subdomain",
        "domain_three",
    }


def test_get_nonexistent_path(package_trie):
    assert package_trie.get("fakepath") is None


def test_get_nonexistent_empty_path():
    trie = PackageTrie()
    assert trie.get("") is None


def test_get_actual_path(package_trie):
    assert package_trie.get("domain_one") is not None


def test_insert_empty_path(test_config):
    trie = PackageTrie()
    trie.insert(test_config, "", [])
    assert set((node.full_path for node in trie)) == {""}


def test_insert_single_level_path(test_config):
    trie = PackageTrie()
    trie.insert(test_config, "domain", [])
    assert set((node.full_path for node in trie)) == {"domain"}


def test_insert_multi_level_path(test_config):
    trie = PackageTrie()
    trie.insert(test_config, "domain.subdomain", [])
    assert set((node.full_path for node in trie)) == {"domain.subdomain"}


def test_find_nearest_at_root(package_trie):
    package = package_trie.find_nearest("other_domain")
    assert package is None


def test_find_nearest_in_single_domain(package_trie):
    package = package_trie.find_nearest("domain_one.thing")
    assert package.full_path == "domain_one"


def test_find_nearest_in_nested_domain(package_trie):
    package = package_trie.find_nearest("domain_two.subdomain.thing")
    assert package.full_path == "domain_two.subdomain"
