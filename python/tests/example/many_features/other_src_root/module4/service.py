# tach-ignore
from module2.service import Module2Service

# tach-ignore(ok)
from module1 import (
    a,  # tach-ignore(ok)
    b,
    c,
    d,
    e,
    f,
    g,  # tach-ignore(ok)
    h,
    i,
    j,
)

from globbed.other.module import something
