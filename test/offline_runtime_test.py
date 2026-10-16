"""Exercise actual kernel restrictions, including inheritance into subprocesses."""
import errno
import os
from pathlib import Path
import socket
import subprocess
import sys
import unittest

RUNNER = Path(__file__).resolve().parents[1] / "scripts/offline-exec.py"


class OfflineRuntimeTests(unittest.TestCase):
    def run_offline(self, code, **kwargs):
        return subprocess.run([sys.executable, str(RUNNER), sys.executable, "-c", code],
                              capture_output=True, text=True, **kwargs)

    def test_ipv4_and_ipv6_socket_creation_is_denied(self):
        for family in (socket.AF_INET, socket.AF_INET6):
            with self.subTest(family=family):
                result = self.run_offline(f'import socket,errno\ntry: socket.socket({int(family)})\nexcept OSError as e: assert e.errno==errno.EPERM\nelse: raise AssertionError("socket allowed")')
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_filter_survives_subprocess_exec(self):
        child = 'import socket,errno\ntry: socket.socket()\nexcept OSError as e: assert e.errno==errno.EPERM\nelse: raise AssertionError("socket allowed")'
        result = self.run_offline(f'import subprocess,sys; subprocess.run([sys.executable,"-c",{child!r}],check=True)')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_inherited_socket_is_closed(self):
        left, right = socket.socketpair()
        try:
            fd = left.fileno()
            result = self.run_offline(f'import os,errno\ntry: os.fstat({fd})\nexcept OSError as e: assert e.errno==errno.EBADF\nelse: raise AssertionError("socket inherited")', pass_fds=(fd,))
            self.assertEqual(result.returncode, 0, result.stderr)
        finally:
            left.close()
            right.close()

    def test_child_exit_status_is_preserved(self):
        self.assertEqual(self.run_offline('raise SystemExit(23)').returncode, 23)


if __name__ == "__main__":
    unittest.main()
