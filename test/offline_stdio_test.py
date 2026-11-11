"""Prevent task data escaping via stdio socket descriptors and their children."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("stdio_probe", ROOT / "scripts/probe-offline-stdio.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class OfflineStdioTests(unittest.TestCase):
    def test_stdio_sockets_fail_closed_before_command_execution(self):
        report = module.probe(ROOT)
        for case in report["cases"]:
            with self.subTest(case=case["id"]):
                self.assertTrue(case["pass"], case)


if __name__ == "__main__":
    unittest.main()
