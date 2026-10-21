import pdfrune


def test_version():
    assert pdfrune.__version__ == "0.1.0"
    assert pdfrune.version() == "0.1.0"
