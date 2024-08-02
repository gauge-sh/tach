from myorg import pack_a, pack_b, pack_c, pack_d, pack_e, pack_f
from myorg.pack_g import hello_world


def test_package():
    assert hello_world() == "Hello world from package G!"


def test_integration():
    assert pack_a.hello_world() == "Hello world from package A!"
    assert pack_b.hello_world() == "Hello world from package B!"
    assert pack_c.hello_world() == "Hello world from package C!"
    assert pack_d.hello_world() == "Hello world from package D!"
    assert pack_e.hello_world() == "Hello world from package E!"
    assert pack_f.hello_world() == "Hello world from package F!"
