import pytest
from modguard.check import check, ErrorInfo, check_import
from modguard.core import BoundaryTrie, PublicMember


@pytest.fixture
def boundary_trie() -> BoundaryTrie:
    trie = BoundaryTrie()
    trie.insert("")
    trie.insert("domain_one")
    trie.insert("domain_two")
    trie.insert("domain_three")
    trie.insert("domain_four", [PublicMember(name="domain_four.public_api")])
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


def test_check_import_across_boundary_public(boundary_trie):
    assert (
        _test_check_import(
            boundary_trie,
            file_mod_path="domain_one",
            import_mod_path="domain_four.public_api",
        )
        is None
    )


def test_check_import_within_boundary(boundary_trie):
    assert (
        _test_check_import(
            boundary_trie,
            file_mod_path="domain_one",
            import_mod_path="domain_one.private_api",
        )
        is None
    )


def test_check_import_external_module(boundary_trie):
    assert (
        _test_check_import(
            boundary_trie,
            file_mod_path="domain_one",
            import_mod_path="external_domain",
        )
        is None
    )


def test_check_import_across_boundary_private(boundary_trie):
    assert (
        _test_check_import(
            boundary_trie,
            file_mod_path="domain_one",
            import_mod_path="domain_four.private_api",
        )
        is not None
    )


def test_check_example_dir_end_to_end():
    expected_errors = [
        ErrorInfo(
            import_mod_path="example.domain_one.interface.domain_one_interface",
            location="example/__init__.py",
            boundary_path="example.domain_one",
        ),
        ErrorInfo(
            import_mod_path="example.domain_three.api.PublicForDomainTwo",
            location="example/__init__.py",
            boundary_path="example.domain_three",
        ),
        ErrorInfo(
            import_mod_path="example.domain_one.interface.domain_one_interface",
            location="example/domain_three/__init__.py",
            boundary_path="example.domain_one",
        ),
        ErrorInfo(
            import_mod_path="example.domain_four.subsystem.private_subsystem_call",
            location="example/__init__.py",
            boundary_path="example.domain_four.subsystem",
        ),
        ErrorInfo(
            location="example/__init__.py",
            import_mod_path="example.domain_one.interface.domain_one_var",
            boundary_path="example.domain_one",
        ),
        ErrorInfo(
            location="example/__init__.py",
            import_mod_path="example.domain_five.inner.private",
            boundary_path="example.domain_five",
        ),
        ErrorInfo(
            location="example/domain_three/__init__.py",
            import_mod_path="example.domain_one.interface.domain_one_var",
            boundary_path="example.domain_one",
        ),
    ]
    check_results = check("example")

    for expected_error in expected_errors:
        assert (
            expected_error in check_results
        ), f"Missing error: {expected_error.message}"
        check_results.remove(expected_error)
    assert len(check_results) == 0, "\n".join(
        (result.message for result in check_results)
    )
