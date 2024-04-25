import pytest

from modguard.core import ModuleTrie, ModuleNode, ModuleConfig


@pytest.fixture
def test_config() -> ModuleConfig:
    return ModuleConfig(scopes=["test"], strict=False)


@pytest.fixture
def module_trie() -> ModuleTrie:
    return ModuleTrie(
        root=ModuleNode(
            is_end_of_path=False,
            full_path="",
            config=None,
            children={
                "domain_one": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_one",
                    config=ModuleConfig(scopes=["test"], strict=False),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_one.subdomain",
                            config=ModuleConfig(scopes=["test"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_two": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_two",
                    config=ModuleConfig(scopes=["test"], strict=False),
                    children={
                        "subdomain": ModuleNode(
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                            config=ModuleConfig(scopes=["test"], strict=False),
                            children={},
                        )
                    },
                ),
                "domain_three": ModuleNode(
                    is_end_of_path=True,
                    full_path="domain_three",
                    config=ModuleConfig(scopes=["test"], strict=False),
                    children={},
                ),
            },
        )
    )


def test_iterate_over_empty_trie():
    assert list(ModuleTrie()) == []


def test_iterate_over_populated_trie(module_trie):
    assert set((node.full_path for node in module_trie)) == {
        "domain_one",
        "domain_one.subdomain",
        "domain_two",
        "domain_two.subdomain",
        "domain_three",
    }


def test_get_nonexistent_path(module_trie):
    assert module_trie.get("fakepath") is None


def test_get_nonexistent_empty_path():
    trie = ModuleTrie()
    assert trie.get("") is None


def test_get_actual_path(module_trie):
    assert module_trie.get("domain_one") is not None


def test_insert_empty_path(test_config):
    trie = ModuleTrie()
    trie.insert(test_config, "")
    assert set((node.full_path for node in trie)) == {""}


def test_insert_single_level_path(test_config):
    trie = ModuleTrie()
    trie.insert(test_config, "domain")
    assert set((node.full_path for node in trie)) == {"domain"}


def test_insert_multi_level_path(test_config):
    trie = ModuleTrie()
    trie.insert(test_config, "domain.subdomain")
    assert set((node.full_path for node in trie)) == {"domain.subdomain"}


def test_find_nearest_at_root(module_trie):
    module = module_trie.find_nearest("other_domain")
    assert module is None


def test_find_nearest_in_single_domain(module_trie):
    module = module_trie.find_nearest("domain_one.thing")
    assert module.full_path == "domain_one"


def test_find_nearest_in_nested_domain(module_trie):
    module = module_trie.find_nearest("domain_two.subdomain.thing")
    assert module.full_path == "domain_two.subdomain"
