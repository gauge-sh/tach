import pytest


from .test_boundary_trie import boundary_trie

from modguard.show import show


def test_show(boundary_trie):
    assert (
        show(boundary_trie)
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
root:
  is_boundary: true
"""
    )
