from myorg.pack_a import hello_world


def test_package():
    assert hello_world() == "Hello world from package A!"
