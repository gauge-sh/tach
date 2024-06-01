# Copyright (c) Microsoft Corporation. All rights reserved.
# Licensed under the MIT License.
"""Light-weight JSON-RPC over standard IO."""

from __future__ import annotations

import atexit
import contextlib
import json
import pathlib
import subprocess
import threading
import uuid
from concurrent.futures import ThreadPoolExecutor
from typing import TYPE_CHECKING, BinaryIO, Sequence

if TYPE_CHECKING:
    import io

CONTENT_LENGTH = "Content-Length: "
RUNNER_SCRIPT = str(pathlib.Path(__file__).parent / "lsp_runner.py")


def to_str(text: str | bytes) -> str:
    """Convert bytes to string as needed."""
    return text.decode("utf-8") if isinstance(text, bytes) else text  # pyright: ignore [reportReturnType]


class StreamClosedException(Exception):
    """JSON RPC stream is closed."""

    pass


class JsonWriter:
    """Manages writing JSON-RPC messages to the writer stream."""

    def __init__(self, writer: io.TextIOWrapper | BinaryIO):
        self._writer = writer
        self._lock = threading.Lock()

    def close(self):
        """Closes the underlying writer stream."""
        with self._lock:
            if not self._writer.closed:
                self._writer.close()

    def write(self, data):
        """Writes given data to stream in JSON-RPC format."""
        if self._writer.closed:
            raise StreamClosedException()

        with self._lock:
            content = json.dumps(data)
            length = len(content.encode("utf-8"))
            self._writer.write(
                str(f"{CONTENT_LENGTH}{length}\r\n\r\n{content}".encode())  # pyright: ignore
            )
            self._writer.flush()


class JsonReader:
    """Manages reading JSON-RPC messages from stream."""

    def __init__(self, reader: io.TextIOWrapper | BinaryIO):
        self._reader = reader

    def close(self):
        """Closes the underlying reader stream."""
        if not self._reader.closed:
            self._reader.close()

    def read(self):
        """Reads data from the stream in JSON-RPC format."""
        if self._reader.closed:
            raise StreamClosedException
        length = None
        while not length:
            line = to_str(self._readline())
            if line.startswith(CONTENT_LENGTH):
                length = int(line[len(CONTENT_LENGTH) :])

        line = to_str(self._readline()).strip()
        while line:
            line = to_str(self._readline()).strip()

        content = to_str(self._reader.read(length))
        return json.loads(content)

    def _readline(self):
        line = self._reader.readline()
        if not line:
            raise EOFError
        return line


class JsonRpc:
    """Manages sending and receiving data over JSON-RPC."""

    def __init__(
        self, reader: io.TextIOWrapper | BinaryIO, writer: io.TextIOWrapper | BinaryIO
    ):
        self._reader = JsonReader(reader)
        self._writer = JsonWriter(writer)

    def close(self):
        """Closes the underlying streams."""
        with contextlib.suppress(Exception):
            self._reader.close()
        with contextlib.suppress(Exception):
            self._writer.close()

    def send_data(self, data):
        """Send given data in JSON-RPC format."""
        self._writer.write(data)

    def receive_data(self):
        """Receive data in JSON-RPC format."""
        return self._reader.read()


def create_json_rpc(readable: BinaryIO, writable: BinaryIO) -> JsonRpc:
    """Creates JSON-RPC wrapper for the readable and writable streams."""
    return JsonRpc(readable, writable)


class ProcessManager:
    """Manages sub-processes launched for running tools."""

    def __init__(self):
        self._args: dict[str, Sequence[str]] = {}
        self._processes: dict[str, subprocess.Popen] = {}
        self._rpc: dict[str, JsonRpc] = {}
        self._lock = threading.Lock()
        self._thread_pool = ThreadPoolExecutor(10)

    def stop_all_processes(self):
        """Send exit command to all processes and shutdown transport."""
        for i in self._rpc.values():
            with contextlib.suppress(Exception):
                i.send_data({"id": str(uuid.uuid4()), "method": "exit"})
        self._thread_pool.shutdown(wait=False)

    def start_process(self, workspace: str, args: Sequence[str], cwd: str) -> None:
        """Starts a process and establishes JSON-RPC communication over stdio."""

        proc = subprocess.Popen(
            args,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stdin=subprocess.PIPE,
        )
        self._processes[workspace] = proc
        self._rpc[workspace] = create_json_rpc(proc.stdout, proc.stdin)  # pyright: ignore

        def _monitor_process():
            proc.wait()
            with self._lock:
                try:
                    del self._processes[workspace]
                    rpc = self._rpc.pop(workspace)
                    rpc.close()

                except:  # noqa: E722
                    pass

        self._thread_pool.submit(_monitor_process)

    def get_json_rpc(self, workspace: str) -> JsonRpc:
        """Gets the JSON-RPC wrapper for the a given id."""
        with self._lock:
            if workspace in self._rpc:
                return self._rpc[workspace]
        raise StreamClosedException()


_process_manager = ProcessManager()
atexit.register(_process_manager.stop_all_processes)


def shutdown_json_rpc():
    """Shutdown all JSON-RPC processes."""
    _process_manager.stop_all_processes()
