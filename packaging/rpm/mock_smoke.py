#!/usr/bin/env python3
import json
import pathlib
import socket
import subprocess
import sys
import tempfile
import time


def read_response(stream, request_id):
    while True:
        line = stream.readline()
        if not line:
            raise RuntimeError("fangd closed the connection before responding")
        message = json.loads(line)
        if message.get("id") == request_id:
            return message


def main():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    with tempfile.TemporaryDirectory(prefix="fangd-rpm-smoke-") as directory:
        state = pathlib.Path(directory) / "state.json"
        process = subprocess.Popen(
            [
                "/usr/bin/fangd",
                "--mock",
                "--tcp",
                f"127.0.0.1:{port}",
                "--state",
                str(state),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        try:
            connection = None
            for _ in range(50):
                try:
                    connection = socket.create_connection(("127.0.0.1", port), timeout=1)
                    break
                except OSError:
                    if process.poll() is not None:
                        raise RuntimeError(process.stderr.read().decode())
                    time.sleep(0.1)
            if connection is None:
                raise RuntimeError("installed fangd did not listen within five seconds")

            with connection:
                stream = connection.makefile("rwb")
                stream.write(b'{"id":1,"cmd":"get_status"}\n')
                stream.flush()
                response = read_response(stream, 1)
                if response.get("ok") is not True or response.get("data", {}).get("mock") is not True:
                    raise RuntimeError(f"unexpected response: {response}")
        finally:
            if process.poll() is None:
                process.terminate()
            try:
                status = process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise RuntimeError("installed fangd ignored SIGTERM")
            if status != 0 and sys.exc_info()[0] is None:
                raise RuntimeError(f"installed fangd exited with {status}: {process.stderr.read().decode()}")


if __name__ == "__main__":
    main()
