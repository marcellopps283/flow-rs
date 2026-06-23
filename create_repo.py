import subprocess
import os

env = os.environ.copy()
if "GITHUB_TOKEN" in env:
    del env["GITHUB_TOKEN"]

cmd = [
    "gh", "repo", "create", "marcellopps283/flow-rs",
    "--public", "--source", ".", "--remote", "origin", "--push",
    "--description", "Flow 2.0: High-performance, native Rust version of Flow (Dynamic Island AI Dictation)"
]

print("Running command:", cmd)
res = subprocess.run(cmd, env=env, capture_output=True, text=True)
print("Return code:", res.returncode)
print("STDOUT:", res.stdout)
print("STDERR:", res.stderr)
