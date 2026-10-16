#!/usr/bin/env python3
"""Execute a Linux command with kernel-enforced socket and io_uring denial."""
import ctypes
import ctypes.util
import errno
import os
import stat
import sys


def main():
    if sys.platform != "linux" or len(sys.argv) < 2:
        raise SystemExit("Usage (Linux only): offline-exec.py COMMAND [ARGS...]")
    library = ctypes.util.find_library("seccomp")
    if not library:
        raise SystemExit("libseccomp is required; refusing to run with network access")
    seccomp = ctypes.CDLL(library, use_errno=True)
    seccomp.seccomp_init.argtypes = [ctypes.c_uint32]
    seccomp.seccomp_init.restype = ctypes.c_void_p
    seccomp.seccomp_syscall_resolve_name.argtypes = [ctypes.c_char_p]
    seccomp.seccomp_syscall_resolve_name.restype = ctypes.c_int
    seccomp.seccomp_rule_add.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int, ctypes.c_uint]
    seccomp.seccomp_rule_add.restype = ctypes.c_int
    seccomp.seccomp_load.argtypes = [ctypes.c_void_p]
    seccomp.seccomp_load.restype = ctypes.c_int
    seccomp.seccomp_release.argtypes = [ctypes.c_void_p]
    context = seccomp.seccomp_init(0x7FFF0000)  # SCMP_ACT_ALLOW
    if not context:
        raise SystemExit("Unable to initialize seccomp")
    try:
        # Deny new sockets and inherited-socket operations. Also deny io_uring,
        # whose asynchronous networking could otherwise avoid connect/send syscalls.
        for name in ("socket", "connect", "sendto", "sendmsg", "sendmmsg", "io_uring_setup"):
            number = seccomp.seccomp_syscall_resolve_name(name.encode())
            if number < 0 or seccomp.seccomp_rule_add(context, 0x50000 | errno.EPERM, number, 0) != 0:
                raise SystemExit(f"Cannot deny {name}; refusing to execute")
        # Remove any non-stdio socket inherited from the launcher.
        for item in os.listdir("/proc/self/fd"):
            descriptor = int(item)
            if descriptor > 2:
                try:
                    if stat.S_ISSOCK(os.fstat(descriptor).st_mode):
                        os.close(descriptor)
                except OSError:
                    pass
        if seccomp.seccomp_load(context) != 0:
            raise SystemExit("Cannot install seccomp filter; refusing to execute")
    finally:
        seccomp.seccomp_release(context)
    os.execvp(sys.argv[1], sys.argv[1:])


if __name__ == "__main__":
    main()
