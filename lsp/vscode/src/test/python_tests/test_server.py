# Copyright (c) Microsoft Corporation. All rights reserved.
# Licensed under the MIT License.
"""
Test for linting over LSP.
"""

from __future__ import annotations

from threading import Event

import pytest
from hamcrest import assert_that, is_

from .lsp_test_client import constants, defaults, session, utils

SERVER_INFO = utils.get_server_info_defaults()
TIMEOUT = 10  # 10 seconds


@pytest.mark.parametrize(
    "test_file_path, expected",
    [
        (
            constants.TEST_DATA / "sample1" / "sample.py",
            {
                "uri": utils.as_uri(str(constants.TEST_DATA / "sample1" / "sample.py")),
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 2, "character": 0},
                            "end": {"line": 2, "character": 99999},
                        },
                        "message": "Cannot import 'sample2.sample2.SAMPLE2'. Tags ['sample1'] "
                        "cannot depend on ['sample2'].",
                        "severity": 1,
                        "source": SERVER_INFO["module"],
                    },
                ],
            },
        ),
        (
            constants.TEST_DATA / "sample2" / "sample2.py",
            {
                "uri": utils.as_uri(
                    str(constants.TEST_DATA / "sample2" / "sample2.py")
                ),
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 2, "character": 0},
                            "end": {"line": 2, "character": 99999},
                        },
                        "message": "Package 'sample1' is in strict mode. Only imports from the "
                        "root of this package are allowed. The import "
                        "'sample1.sample.SAMPLE1' (in 'sample2.sample2') is not "
                        "included in __all__.",
                        "severity": 1,
                        "source": SERVER_INFO["module"],
                    },
                ],
            },
        ),
    ],
)
def test_import_example(test_file_path, expected):
    """Test to linting on file open."""
    test_file_uri = utils.as_uri(str(test_file_path))

    contents = test_file_path.read_text()

    actual = []
    with session.LspSession() as ls_session:
        ls_session.initialize(defaults.VSCODE_DEFAULT_INITIALIZE)

        done = Event()

        def _handler(params):
            nonlocal actual
            actual = params
            done.set()

        ls_session.set_notification_callback(session.PUBLISH_DIAGNOSTICS, _handler)

        ls_session.notify_did_open(
            {
                "textDocument": {
                    "uri": test_file_uri,
                    "languageId": "python",
                    "version": 1,
                    "text": contents,
                }
            }
        )
        # wait for some time to receive all notifications
        done.wait(TIMEOUT)

    assert_that(actual, is_(expected))
