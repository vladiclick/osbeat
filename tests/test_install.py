"""Offline integration tests: the installer must never trust an archive first."""
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin with spaces"
        self.archive = self.root / "osbeat-v0.2.0-aarch64-apple-darwin.tar.gz"
        self.checksum = self.root / "SHA256SUMS"
        self.write_archive()

    def tearDown(self):
        self.temp.cleanup()

    def write_archive(self, extra=None, symlink=False):
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, data in [("osbeat", b"#!/bin/sh\necho fixture\n"),
                               ("LICENSE", b"MIT\n"),
                               ("THIRD_PARTY_NOTICES", b"credits\n")]:
                info = tarfile.TarInfo(name)
                info.size = len(data)
                info.mode = 0o755 if name == "osbeat" else 0o644
                if symlink and name == "osbeat":
                    info.type = tarfile.SYMTYPE
                    info.linkname = "/bin/sh"
                    info.size = 0
                archive.addfile(info, io.BytesIO(data))
            if extra:
                info = tarfile.TarInfo(extra)
                info.size = 3
                archive.addfile(info, io.BytesIO(b"bad"))
        self.checksum.write_text(
            hashlib.sha256(self.archive.read_bytes()).hexdigest()
            + "  " + self.archive.name + "\n"
        )

    def run_installer(self, *args, offline=True, env=None):
        arguments = ["sh", str(ROOT / "install.sh"), "--bin-dir", str(self.bin)]
        if offline:
            arguments += ["--archive", str(self.archive), "--checksum", str(self.checksum)]
        return subprocess.run(arguments + list(args), text=True, capture_output=True,
                              env={**os.environ, **(env or {})})

    def assert_rejected(self, *args, offline=True):
        self.bin.mkdir(exist_ok=True)
        binary = self.bin / "osbeat"
        binary.write_text("old binary")
        result = self.run_installer(*args, offline=offline)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(binary.read_text(), "old binary")
        self.assertEqual(sorted(p.name for p in self.bin.iterdir()), ["osbeat"])
        return result

    def test_verified_install_is_executable(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        binary = self.bin / "osbeat"
        self.assertTrue(os.access(binary, os.X_OK))
        self.assertEqual(subprocess.check_output([binary], text=True), "fixture\n")
        self.assertIn(str(self.bin), result.stdout)

    def test_corruption_preserves_existing_binary(self):
        self.archive.write_bytes(self.archive.read_bytes() + b"corrupt")
        self.assert_rejected()

    def test_ambiguous_checksum_is_rejected(self):
        self.checksum.write_text(self.checksum.read_text() * 2)
        self.assert_rejected()

    def test_wrong_checksum_name_is_rejected(self):
        self.checksum.write_text(self.checksum.read_text().replace(self.archive.name, "other.tar.gz"))
        self.assert_rejected()

    def test_malformed_checksum_is_rejected(self):
        self.checksum.write_text("invalid  " + self.archive.name + "\n")
        self.assert_rejected()

    def test_traversal_is_rejected_without_extracting(self):
        self.write_archive(extra="../escape")
        self.assert_rejected()
        self.assertFalse((self.root / "escape").exists())

    def test_absolute_path_is_rejected(self):
        self.write_archive(extra="/tmp/osbeat-should-not-exist")
        self.assert_rejected()

    def test_symlink_binary_is_rejected(self):
        self.write_archive(symlink=True)
        self.assert_rejected()

    def test_duplicate_archive_member_is_rejected(self):
        self.write_archive(extra="osbeat")
        self.assert_rejected()

    def test_archive_requires_checksum(self):
        result = self.run_installer("--archive", str(self.archive), offline=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.bin.exists())

    def test_default_repository_is_osbeat(self):
        result = self.run_installer("--help", offline=False)
        self.assertIn("vladiclick/osbeat", result.stdout)

    def test_other_platforms_are_rejected_before_network(self):
        fake = self.root / "fake-bin"
        fake.mkdir()
        for name in ["uname", "curl"]:
            tool = fake / name
            tool.write_text('#!/bin/sh\n[ "$1" = -s ] && echo FreeBSD && exit 0\nexit 99\n')
            tool.chmod(0o755)
        result = self.run_installer(offline=False, env={"PATH": f"{fake}:{os.environ['PATH']}"})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("macOS only", result.stderr)

    def test_repo_and_version_are_validated_before_network(self):
        for args in [("--repo", "../bad"), ("--repo", "https://github.com/a/b"),
                     ("--repo", "a/b", "--version", "../v0.2.0"),
                     ("--repo", "a/b", "--version", "v0.2.0;echo bad"),
                     ("--repo", "a/b\nx/y"),
                     ("--repo", "a/b", "--version", "v0.2.0\nv0.3.0")]:
            with self.subTest(args=args):
                self.assert_rejected(*args, offline=False)

    def test_help_and_unknown_argument(self):
        self.assertEqual(self.run_installer("--help", offline=False).returncode, 0)
        self.assertNotEqual(self.run_installer("--oops", offline=False).returncode, 0)


class PackagingTests(unittest.TestCase):
    def test_package_round_trips_through_installer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "osbeat"
            binary.write_text("#!/bin/sh\necho packaged\n")
            result = subprocess.run(
                ["sh", str(ROOT / "scripts/package_release.sh"), "--target",
                 "aarch64-apple-darwin", "--version", "v0.2.0", "--binary", str(binary),
                 "--out-dir", str(root / "release")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            archive = root / "release/osbeat-v0.2.0-aarch64-apple-darwin.tar.gz"
            with tarfile.open(archive) as package:
                self.assertEqual(sorted(package.getnames()), ["LICENSE", "THIRD_PARTY_NOTICES", "osbeat"])
                self.assertEqual(package.extractfile("osbeat").read(), binary.read_bytes())
            result = subprocess.run(
                ["sh", str(ROOT / "install.sh"), "--archive", str(archive), "--checksum",
                 str(archive) + ".sha256", "--bin-dir", str(root / "installed")],
                capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(subprocess.check_output([root / "installed/osbeat"], text=True), "packaged\n")

    def test_package_refuses_tag_version_mismatch(self):
        result = subprocess.run(
            ["sh", str(ROOT / "scripts/package_release.sh"), "--target",
             "aarch64-apple-darwin", "--version", "v99.0.0", "--binary", "/bin/sh"],
            capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cargo.toml", result.stderr)

    def test_local_install_preserves_paths_and_uses_locked_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cargo = root / "cargo"
            log = root / "arguments"
            cargo.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$INSTALL_TEST_LOG"\n')
            cargo.chmod(0o755)
            prefix = root / "local prefix"
            result = subprocess.run(
                ["sh", str(ROOT / "scripts/install_local.sh"), "--prefix", str(prefix)],
                capture_output=True, text=True,
                env={**os.environ, "PATH": str(root) + ":" + os.environ["PATH"],
                     "INSTALL_TEST_LOG": str(log)})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(log.read_text().splitlines(),
                             ["install", "--locked", "--path", str(ROOT), "--root", str(prefix), "--force"])
