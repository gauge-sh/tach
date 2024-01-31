import pytest
from modguard.base import guard



@guard(deny=['tests.test_guard'])
def func_one():
    pass


def func_two():
    func_one()



def test_func_one_not_allowed_in_func_two():
    with pytest.raises(RuntimeError):
        func_two()