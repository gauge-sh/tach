from __future__ import annotations

import time

import pytest

from tach.check import check


@pytest.mark.parametrize("test_input", [1, 2, 3, 4, 5])
def test_slow(test_input):
    time.sleep(0.5)
    print(check)
