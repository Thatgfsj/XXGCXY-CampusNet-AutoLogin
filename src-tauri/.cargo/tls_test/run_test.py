import subprocess, sys

tests = ["test_simple_exit.exe", "test_tls_exit.exe", "test_tls_thread.exe", "test_linker_tls.exe"]
for t in tests:
    try:
        r = subprocess.run([t], capture_output=True, timeout=5)
        print(f"{t}: rc={r.returncode}, stderr={r.stderr}")
    except Exception as e:
        print(f"{t}: ERROR - {e}")

