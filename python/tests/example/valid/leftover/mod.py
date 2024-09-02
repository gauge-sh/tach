from __future__ import annotations
import typing as t

from domain_one import x

if t.TYPE_CHECKING:
    from domain_three import x as ignored
    ignored


x
