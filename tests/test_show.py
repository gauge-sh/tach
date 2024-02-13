import pytest

from modguard.show import show
from modguard.core import BoundaryTrie, PublicMember, BoundaryNode


@pytest.fixture
def boundary_trie():
    return BoundaryTrie(
        root=BoundaryNode(
            public_members={},
            children={
                "domain_one": BoundaryNode(
                    public_members={},
                    children={},
                    is_end_of_path=True,
                    full_path="domain_one",
                ),
                "domain_two": BoundaryNode(
                    public_members={},
                    children={
                        "subdomain": BoundaryNode(
                            public_members={},
                            children={},
                            is_end_of_path=True,
                            full_path="domain_two.subdomain",
                        )
                    },
                    is_end_of_path=True,
                    full_path="domain_two",
                ),
                "domain_three": BoundaryNode(
                    public_members={},
                    children={},
                    is_end_of_path=True,
                    full_path="domain_three",
                ),
                "domain_four": BoundaryNode(
                    public_members={
                        "domain_four.public_api": PublicMember(
                            name="domain_four.public_api", allowlist=None
                        )
                    },
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
        == """domain_four:
  is_boundary: true
  public_api:
    is_public: true
domain_one:
  is_boundary: true
domain_three:
  is_boundary: true
domain_two:
  is_boundary: true
  subdomain:
    is_boundary: true
"""
    )
