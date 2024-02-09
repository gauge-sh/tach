import pytest

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


def test_iterate_over_empty_trie():
    assert list(BoundaryTrie()) == []


def test_iterate_over_populated_trie(boundary_trie):
    assert set((node.full_path for node in boundary_trie)) == {
        "",
        "domain_one",
        "domain_two",
        "domain_two.subdomain",
        "domain_three",
        "domain_four",
    }


def test_get_nonexistent_path(boundary_trie):
    assert boundary_trie.get("fakepath") is None


def test_get_nonexistent_empty_path():
    trie = BoundaryTrie()
    assert trie.get("") is None


def test_get_actual_path(boundary_trie):
    assert boundary_trie.get("domain_one") is not None


def test_get_actual_empty_path(boundary_trie):
    assert boundary_trie.get("") is not None


def test_insert_empty_path():
    trie = BoundaryTrie()
    trie.insert("")
    assert set((node.full_path for node in trie)) == {""}


def test_insert_single_level_path():
    trie = BoundaryTrie()
    trie.insert("domain")
    assert set((node.full_path for node in trie)) == {"domain"}


def test_insert_multi_level_path():
    trie = BoundaryTrie()
    trie.insert("domain.subdomain")
    assert set((node.full_path for node in trie)) == {"domain.subdomain"}


def test_register_unnamed_public_member_at_root(boundary_trie):
    pub_member = PublicMember(name="")
    boundary_trie.register_public_member("member_path", pub_member)
    assert boundary_trie.get("").public_members == {"member_path": pub_member}


def test_register_named_public_member_at_root(boundary_trie):
    pub_member = PublicMember(name="member_variable")
    boundary_trie.register_public_member("member_path", pub_member)
    assert boundary_trie.get("").public_members == {
        "member_path.member_variable": pub_member
    }


def test_register_named_public_member_at_single_level_domain(boundary_trie):
    pub_member = PublicMember(name="member_variable")
    boundary_trie.register_public_member("domain_one", pub_member)
    assert boundary_trie.get("domain_one").public_members == {
        "domain_one.member_variable": pub_member
    }


def test_register_unnamed_public_member_at_single_level_domain(boundary_trie):
    pub_member = PublicMember(name="")
    boundary_trie.register_public_member("domain_one", pub_member)
    assert boundary_trie.get("domain_one").public_members == {"domain_one": pub_member}


def test_register_named_public_member_at_nested_domain(boundary_trie):
    pub_member = PublicMember(name="member_variable")
    boundary_trie.register_public_member("domain_two.subdomain", pub_member)
    assert boundary_trie.get("domain_two.subdomain").public_members == {
        "domain_two.subdomain.member_variable": pub_member
    }


def test_register_unnamed_public_member_at_nested_domain(boundary_trie):
    pub_member = PublicMember(name="")
    boundary_trie.register_public_member("domain_two.subdomain", pub_member)
    assert boundary_trie.get("domain_two.subdomain").public_members == {
        "domain_two.subdomain": pub_member
    }


def test_find_nearest_at_root(boundary_trie):
    boundary = boundary_trie.find_nearest("other_domain")
    assert boundary.full_path == ""


def test_find_nearest_in_single_domain(boundary_trie):
    boundary = boundary_trie.find_nearest("domain_one.thing")
    assert boundary.full_path == "domain_one"


def test_find_nearest_in_nested_domain(boundary_trie):
    boundary = boundary_trie.find_nearest("domain_two.subdomain.thing")
    assert boundary.full_path == "domain_two.subdomain"
