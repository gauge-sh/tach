import pytest

from modguard.core import BoundaryTrie, BoundaryNode
from modguard.show import show


@pytest.fixture
def boundary_trie():
    return BoundaryTrie(
        root=BoundaryNode(
            children={
                "domain_one": BoundaryNode(
                    children={},
                    is_end_of_path=True,
                    full_path="domain_one",
                ),
                "domain_two": BoundaryNode(
                    children={
                        "subdomain": BoundaryNode(
                            children={},
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                        )
                    },
                    is_end_of_path=True,
                    full_path="domain_two",
                ),
                "domain_three": BoundaryNode(
                    children={},
                    is_end_of_path=True,
                    full_path="domain_three",
                ),
                "domain_four": BoundaryNode(
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
