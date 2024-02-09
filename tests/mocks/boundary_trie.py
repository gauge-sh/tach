from modguard.core.boundary import BoundaryTrie
from modguard.core.public import PublicMember


def build_example_boundary_trie() -> BoundaryTrie:
    trie = BoundaryTrie()
    trie.insert("")
    trie.insert("domain_one")
    trie.insert("domain_two")
    trie.insert("domain_three")
    trie.insert("domain_four", [PublicMember(name="domain_four.public_api")])
    return trie
