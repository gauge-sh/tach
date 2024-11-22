# type: ignore reportConstantRedefinition
from __future__ import annotations

import sys


def test_unicode_encoding():
    try:
        "✅".encode(sys.stdout.encoding)
        return True
    except UnicodeEncodeError:
        return False


SUPPORTS_UNICODE = test_unicode_encoding()


### Icons which depend on Unicode support
if SUPPORTS_UNICODE:
    SUCCESS = "✅"
    WARNING = "⚠️"
    FAIL = "❌"
else:
    SUCCESS = "[OK]"
    WARNING = "[WARN]"
    FAIL = "[FAIL]"
