import rupdf


def test_version():
    assert rupdf.__version__ == "0.1.0"
    assert rupdf.version() == "0.1.0"
