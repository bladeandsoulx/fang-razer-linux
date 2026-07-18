import importlib.util
import io
import pathlib
import unittest

MODULE_PATH = pathlib.Path(__file__).with_name("mock_smoke.py")
SPEC = importlib.util.spec_from_file_location("mock_smoke", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ResponseTest(unittest.TestCase):
    def test_skips_events_and_returns_matching_response(self):
        stream = io.BytesIO(
            b'{"event":"telemetry","data":{}}\n'
            b'{"id":1,"ok":true,"data":{"mock":true}}\n'
        )
        response = MODULE.read_response(stream, 1)
        self.assertTrue(response["ok"])
        self.assertTrue(response["data"]["mock"])


if __name__ == "__main__":
    unittest.main()
