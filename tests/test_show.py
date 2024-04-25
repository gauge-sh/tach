import pytest

from modguard.core import ModuleTrie, ModuleNode
from modguard.show import show


@pytest.fixture
def boundary_trie():
    return ModuleTrie(
        root=ModuleNode(
            children={
                "domain_one": ModuleNode(
                    children={},
                    is_end_of_path=True,
                    full_path="domain_one",
                ),
                "domain_two": ModuleNode(
                    children={
                        "subdomain": ModuleNode(
                            children={},
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                        )
                    },
                    is_end_of_path=True,
                    full_path="domain_two",
                ),
                "domain_three": ModuleNode(
                    children={},
                    is_end_of_path=True,
                    full_path="domain_three",
                ),
                "domain_four": ModuleNode(
                    children={},
                    is_end_of_path=True,
                    full_path="domain_four",
                ),
            },
            is_end_of_path=True,
            full_path="",
        )
    )


def test_show(boundary_trie):
    assert (
        show(boundary_trie)[0]
        == """domain_one:
  is_boundary: true
domain_two:
  is_boundary: true
  subdomain:
    is_boundary: true
domain_three:
  is_boundary: true
domain_four:
  is_boundary: true
"""
    )
