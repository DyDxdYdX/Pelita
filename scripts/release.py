"""Prepare release metadata and validate/stage Tauri installers."""

import argparse
import json
import re
import shutil
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(
    r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r"(?:-(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*)?"
)
FORMATS = {
    "windows-x86_64": ("nsis", ".exe"),
    "macos-aarch64": ("dmg", ".dmg"),
    "macos-x86_64": ("dmg", ".dmg"),
    "linux-x86_64-appimage": ("appimage", ".AppImage"),
    "linux-x86_64-deb": ("deb", ".deb"),
}


def fail(message):
    raise SystemExit(message)


def version():
    package = json.loads((ROOT / "app/package.json").read_text())
    package_lock = json.loads((ROOT / "app/package-lock.json").read_text())
    config = json.loads((ROOT / "app/src-tauri/tauri.conf.json").read_text())
    cargo = tomllib.loads((ROOT / "app/src-tauri/Cargo.toml").read_text())
    cargo_lock = tomllib.loads((ROOT / "app/src-tauri/Cargo.lock").read_text())
    locked_cargo = [p["version"] for p in cargo_lock["package"] if p["name"] == cargo["package"]["name"]]
    values = {
        "npm": package["version"],
        "npm lock": package_lock["version"],
        "npm lock root": package_lock["packages"][""]["version"],
        "Tauri": config["version"],
        "Cargo": cargo["package"]["version"],
        "Cargo lock": locked_cargo[0] if len(locked_cargo) == 1 else None,
    }
    if len(set(values.values())) != 1:
        fail(f"Application versions differ: {values}")
    found = package["version"]
    if not SEMVER.fullmatch(found):
        fail(f"Invalid release version: {found}")
    return found


def prepare(new_version):
    if not SEMVER.fullmatch(new_version):
        fail(f"Invalid release version: {new_version}")
    old_version = version()
    if new_version == old_version:
        print(f"Already prepared v{new_version}")
        return

    package_path = ROOT / "app/package.json"
    lock_path = ROOT / "app/package-lock.json"
    config_path = ROOT / "app/src-tauri/tauri.conf.json"
    cargo_path = ROOT / "app/src-tauri/Cargo.toml"
    cargo_lock_path = ROOT / "app/src-tauri/Cargo.lock"

    package = json.loads(package_path.read_text())
    package_lock = json.loads(lock_path.read_text())
    config = json.loads(config_path.read_text())
    package["version"] = new_version
    package_lock["version"] = new_version
    package_lock["packages"][""]["version"] = new_version
    config["version"] = new_version

    cargo, cargo_count = re.subn(
        rf'(?m)^(version = "){re.escape(old_version)}(")$',
        rf'\g<1>{new_version}\g<2>',
        cargo_path.read_text(),
    )
    lock_prefix = '[[package]]\nname = "pelita"\nversion = "'
    cargo_lock, lock_count = re.subn(
        rf'(?m)^({re.escape(lock_prefix)}){re.escape(old_version)}(")$',
        rf'\g<1>{new_version}\g<2>',
        cargo_lock_path.read_text(),
    )
    if cargo_count != 1 or lock_count != 1:
        fail(f"Could not identify unique Cargo version entries (manifest={cargo_count}, lock={lock_count})")

    for path, content in (
        (package_path, json.dumps(package, indent=2) + "\n"),
        (lock_path, json.dumps(package_lock, indent=2) + "\n"),
        (config_path, json.dumps(config, indent=2) + "\n"),
        (cargo_path, cargo),
        (cargo_lock_path, cargo_lock),
    ):
        path.write_text(content)
    if version() != new_version:
        fail("Prepared versions did not validate")
    print(f"Prepared v{new_version} in npm, Cargo, and Tauri metadata")


def expected_names(tag):
    return {f"Pelita_{tag}-{platform}{extension}" for platform, (_, extension) in FORMATS.items()}


def stage(tag, platform, source, output):
    if platform == "linux-x86_64":
        keys = ("linux-x86_64-appimage", "linux-x86_64-deb")
    elif platform in FORMATS:
        keys = (platform,)
    else:
        fail(f"Unknown platform: {platform}")
    output.mkdir(parents=True, exist_ok=True)
    for key in keys:
        bundle, extension = FORMATS[key]
        matches = list(source.glob(f"{bundle}/*{extension}"))
        if len(matches) != 1 or not matches[0].is_file() or matches[0].stat().st_size == 0:
            fail(f"Expected exactly one nonempty {key} installer in {source / bundle}; found {matches}")
        destination = output / f"Pelita_{tag}-{key}{extension}"
        shutil.copy2(matches[0], destination)
        print(f"Staged {destination} ({destination.stat().st_size} bytes)")


def verify(tag, directory, names=None):
    expected = expected_names(tag)
    actual = set(names) if names is not None else {p.name for p in directory.iterdir() if p.is_file()}
    if actual != expected:
        fail(f"Release assets differ: missing={sorted(expected - actual)}, extra={sorted(actual - expected)}")
    if names is None:
        for path in directory.iterdir():
            if not path.is_file() or path.stat().st_size == 0:
                fail(f"Empty or invalid release asset: {path}")
    print(f"Verified {len(expected)} release assets for {tag}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("prepare", "check-version", "stage", "verify", "verify-upload"))
    parser.add_argument("--version")
    parser.add_argument("--tag")
    parser.add_argument("--platform")
    parser.add_argument("--source", type=Path)
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        if not args.version:
            fail("prepare requires --version")
        prepare(args.version)
        return
    current = version()
    tag = args.tag or f"v{current}"
    if tag != f"v{current}":
        fail(f"Tag {tag!r} must match application version v{current}")
    if args.command == "check-version":
        print(f"Validated {tag}")
    elif args.command == "stage":
        if not args.platform or not args.source or not args.directory:
            fail("stage requires --platform, --source and --directory")
        stage(tag, args.platform, args.source, args.directory)
    elif args.command == "verify":
        verify(tag, args.directory or fail("verify requires --directory"))
    else:
        names = [name for name in sys.stdin.read().splitlines() if name]
        verify(tag, Path("."), names)


if __name__ == "__main__":
    main()
