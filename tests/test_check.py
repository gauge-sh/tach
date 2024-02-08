import pytest
from modguard.check import check, ErrorInfo, check_import
from modguard.core import BoundaryTrie
from .mocks.boundary_trie import build_example_boundary_trie


@pytest.fixture
def boundary_trie() -> BoundaryTrie:
    return build_example_boundary_trie()


def test_check_import(boundary_trie):
    file_mod_path = "domain_one"
    file_boundary = boundary_trie.find_nearest(file_mod_path)
    assert file_boundary is not None
    import_mod_path = "domain_four.public_api"
    assert (
        check_import(
            boundary_trie=boundary_trie,
            import_mod_path=import_mod_path,
            file_nearest_boundary=file_boundary,
            file_mod_path=file_mod_path,
        )
        is None
    )


def test_check():
    expected_errors = [
        ErrorInfo(
            import_mod_path="example.domain_one.interface.domain_one_interface",
            location="example/__init__.py",
            boundary_path="example.domain_one",
        ),
        ErrorInfo(
            import_mod_path="example.domain_three.api.public_for_domain_two",
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
