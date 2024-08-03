from myorg import pack_a, pack_d, pack_e
from myorg.pack_f import hello_world


def test_package():
    assert hello_world() == "Hello world from package F!"


def test_integration():
    assert pack_a.hello_world() == "Hello world from package A!"
    assert pack_d.hello_world() == "Hello world from package D!"
    assert pack_e.hello_world() == "Hello world from package E!"
