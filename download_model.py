import os
import urllib.request
import tarfile
import ssl
from pathlib import Path

# Disable SSL verification just in case of local cert issues
ssl._create_default_https_context = ssl._create_unverified_context

url = "https://huggingface.co/smcleod/nemotron-3.5-asr-streaming-0.6b-int8/resolve/main/nemotron-3.5-asr-streaming-0.6b-int8.tar.gz"
model_dir = Path("models/nemotron")
tar_path = model_dir / "model.tar.gz"

print(f"Creating directory {model_dir}...")
model_dir.mkdir(parents=True, exist_ok=True)

print(f"Downloading Nemotron INT8 model from HuggingFace (this may take a few minutes for ~450MB)...")
def report(block_num, block_size, total_size):
    downloaded = block_num * block_size
    percent = min(100, int(downloaded * 100 / total_size)) if total_size > 0 else 0
    # Print every 10%
    if downloaded % (10 * 1024 * 1024) < block_size:
        print(f"Downloaded {downloaded / 1024 / 1024:.1f} MB of {total_size / 1024 / 1024:.1f} MB ({percent}%)")

urllib.request.urlretrieve(url, tar_path, reporthook=report)

print("Download complete. Extracting files...")
with tarfile.open(tar_path, "r:gz") as tar:
    tar.extractall(path=model_dir)

print("Extraction complete. Cleaning up tar.gz...")
tar_path.unlink()
print("Model ready!")
