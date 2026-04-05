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

The manifest was captured on April 2, 2026 and verifies each fetched file by
SHA-256 so drift is explicit.

- `abseil/CMakeLists.txt`
  Source: `https://github.com/abseil/abseil-cpp/blob/master/CMakeLists.txt`
- `catch2/CMakeLists.txt`
  Source: `https://github.com/catchorg/Catch2/blob/devel/CMakeLists.txt`
- `cli11/CMakeLists.txt`
  Source: `https://github.com/CLIUtils/CLI11/blob/main/CMakeLists.txt`
- `cmake_cmbzip2/CMakeLists.txt`
  Source: `https://github.com/Kitware/CMake/blob/master/Utilities/cmbzip2/CMakeLists.txt`
- `googletest/CMakeLists.txt`
  Source: `https://github.com/google/googletest/blob/main/CMakeLists.txt`
- `llvm_tablegen/CMakeLists.txt`
  Source: `https://github.com/llvm/llvm-project/blob/main/llvm/utils/TableGen/CMakeLists.txt`
- `nlohmann_json/CMakeLists.txt`
  Source: `https://github.com/nlohmann/json/blob/develop/CMakeLists.txt`
- `opencv_flann/CMakeLists.txt`
  Source: `https://github.com/opencv/opencv/blob/4.x/modules/flann/CMakeLists.txt`
- `protobuf/CMakeLists.txt`
  Source: `https://github.com/protocolbuffers/protobuf/blob/main/CMakeLists.txt`
- `qtbase_network/CMakeLists.txt`
  Source: `https://github.com/qt/qtbase/blob/dev/src/network/CMakeLists.txt`

`monorepo_root.cmake` is a local synthetic fixture and is intentionally not
part of the real-world corpus count.
