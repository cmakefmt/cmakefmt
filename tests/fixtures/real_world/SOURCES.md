# Real-World Fixture Sources

The upstream real-world corpus is no longer checked into the repository.
Instead, `tests/fixtures/real_world/manifest.toml` pins the fixture list and
expected content hashes, and `scripts/fetch-real-world-corpus.py` downloads the
files into `target/real-world-corpus/` on demand.

To populate the local corpus:

```bash
python3 scripts/fetch-real-world-corpus.py
```

To generate local before/after review artefacts:

```bash
scripts/review-real-world-corpus.sh
```

The manifest was captured on April 2, 2026 and now pins each fixture to both:

- an immutable upstream commit hash
- an expected SHA-256 of the fetched file contents

That way branch movement upstream does not silently change the corpus.

- `abseil/CMakeLists.txt`
  Source branch at capture time: `master`
- `catch2/CMakeLists.txt`
  Source branch at capture time: `devel`
- `cli11/CMakeLists.txt`
  Source branch at capture time: `main`
- `cmake_cmbzip2/CMakeLists.txt`
  Source branch at capture time: `master`
- `googletest/CMakeLists.txt`
  Source branch at capture time: `main`
- `llvm_tablegen/CMakeLists.txt`
  Source branch at capture time: `main`
- `nlohmann_json/CMakeLists.txt`
  Source branch at capture time: `develop`
- `opencv_flann/CMakeLists.txt`
  Source branch at capture time: `4.x`
- `protobuf/CMakeLists.txt`
  Source branch at capture time: `main`
- `qtbase_network/CMakeLists.txt`
  Source branch at capture time: `dev`

`monorepo_root.cmake` is a local synthetic fixture and is intentionally not
part of the real-world corpus count.
