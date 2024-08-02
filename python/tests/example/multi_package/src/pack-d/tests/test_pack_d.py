from myorg.pack_d import hello_world


def test_package():
    assert hello_world() == "Hello world from package D!"
