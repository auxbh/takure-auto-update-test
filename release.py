import hashlib
import json
import os
import shutil
import subprocess
import zipfile
from pathlib import Path

import tomllib

CARGO_TOML_PATH = Path("./Cargo.toml")

SIGNING_KEY_PATH = Path("./takure_signing.pfx")
SIGNING_KEY_PASSWORD = os.environ["TAKURE_SIGNING_KEY_PASSWORD"]

# Prebuilt, unsigned DLLs downloaded from the matrixed build job as artifacts.
ARTIFACTS = {
    "takure_32": Path("artifacts/takure_32/takure.dll"),
    "takure_64": Path("artifacts/takure_64/takure.dll"),
}

DIST_FOLDER = Path("dist/")


def sign_executable(key: Path, password: str, file: Path):
    r = subprocess.run(
        [
            "signtool",
            "sign",
            "-f",
            str(key),
            "-p",
            password,
            "-fd",
            "SHA256",
            "-tr",
            "https://timestamp.sectigo.com",
            "-td",
            "SHA256",
            "-v",
            str(file),
        ]
    )
    r.check_returncode()


def sha256_of(file: Path) -> str:
    s = hashlib.sha256()
    with file.open("rb") as f:
        while True:
            d = f.read(1_048_576)
            if not d:
                break
            s.update(d)
    return s.hexdigest()


if not SIGNING_KEY_PATH.exists():
    raise SystemExit(
        f"[ERROR] Signing key not found at {SIGNING_KEY_PATH}. Cannot make a release without it."
    )

with CARGO_TOML_PATH.open("rb") as f:
    cargo_toml = tomllib.load(f)

commit = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
version = cargo_toml["package"]["version"]

shutil.rmtree(DIST_FOLDER, ignore_errors=True)
DIST_FOLDER.mkdir(parents=True, exist_ok=True)

checksums = {}

for artifact_name, dll_path in ARTIFACTS.items():
    if not dll_path.exists():
        raise SystemExit(f"[ERROR] Expected build artifact not found at {dll_path}.")

    print(f"[INFO] Signing {dll_path}...")
    sign_executable(SIGNING_KEY_PATH, SIGNING_KEY_PASSWORD, dll_path)

    checksums[artifact_name] = sha256_of(dll_path)

    archive_path = DIST_FOLDER / f"{artifact_name}.zip"
    with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(dll_path, "takure.dll")
        z.write("takure.toml", "takure.toml")

    print(f"[INFO] Wrote {archive_path}")

update_manifest = {
    "version": version,
    "commit": commit,
    "sha256_32": checksums["takure_32"],
    "sha256_64": checksums["takure_64"],
}

with (DIST_FOLDER / "update.json").open("w", encoding="utf-8") as f:
    json.dump(update_manifest, f, ensure_ascii=False, indent=4)

print("[INFO] Done. Release assets and update.json are in dist/.")
