from myorg import pack_a, pack_b
from myorg.pack_c import hello_world


def test_package():
    assert hello_world() == "Hello world from package C!"


def test_integration():
    assert pack_a.hello_world() == "Hello world from package A!"
    assert pack_b.hello_world() == "Hello world from package B!"
