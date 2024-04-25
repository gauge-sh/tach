import pytest
from modguard.check import check, ErrorInfo, check_import
from modguard.core import BoundaryTrie


@pytest.fixture
def boundary_trie() -> BoundaryTrie:
    trie = BoundaryTrie()
    trie.insert("")
    trie.insert("domain_one")
    trie.insert("domain_two")
    trie.insert("domain_three")
    trie.insert("domain_four")
    return trie


def _test_check_import(
    boundary_trie: BoundaryTrie, file_mod_path: str, import_mod_path: str
):
    file_boundary = boundary_trie.find_nearest(file_mod_path)
    assert file_boundary is not None, f"Couldn't find boundary for {file_mod_path}"
    return check_import(
        boundary_trie=boundary_trie,
        import_mod_path=import_mod_path,
        file_nearest_boundary=file_boundary,
        file_mod_path=file_mod_path,
    )
