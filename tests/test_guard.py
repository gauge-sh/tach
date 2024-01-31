import pytest
from modguard.base import guard



@guard(deny=['func_two'], allow=['func_three'])
def func_one():
    pass


def func_two():
    func_one()

def func_three():
    func_one()

def test_func_one_not_allowed_in_func_two():
    with pytest.raises(RuntimeError):
        func_two()


def test_func_one_allowed_in_func_three():
    func_three()