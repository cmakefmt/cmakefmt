{ lib
, fetchurl
, stdenv
, autoPatchelfHook
, gcc-unwrapped
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "cmakefmt";
  version = "0.2.0";

  src =
    if stdenv.hostPlatform.system == "x86_64-linux" then
      fetchurl {
        url = "https://github.com/cmakefmt/cmakefmt/releases/download/v${finalAttrs.version}/cmakefmt-${finalAttrs.version}-x86_64-unknown-linux-musl.tar.gz";
        hash = "sha256-Ps/6tIO5sz3uBNvAkiQBs2wv99UW5Of2DP+Prmlhmkg=";
      }
    else if stdenv.hostPlatform.system == "aarch64-linux" then
      fetchurl {
        url = "https://github.com/cmakefmt/cmakefmt/releases/download/v${finalAttrs.version}/cmakefmt-${finalAttrs.version}-aarch64-unknown-linux-gnu.tar.gz";
        hash = "sha256-VWLw4MHeghZ8ueMIBDlKqlg7d7euApAEE2vD9yRGPew=";
      }
    else if stdenv.hostPlatform.system == "x86_64-darwin" then
      fetchurl {
        url = "https://github.com/cmakefmt/cmakefmt/releases/download/v${finalAttrs.version}/cmakefmt-${finalAttrs.version}-x86_64-apple-darwin.tar.gz";
        hash = "sha256-4L1cAECf9MMiAUCnwfSc71py5ldUCfeCiJe0BDT+g+s=";
      }
    else if stdenv.hostPlatform.system == "aarch64-darwin" then
      fetchurl {
        url = "https://github.com/cmakefmt/cmakefmt/releases/download/v${finalAttrs.version}/cmakefmt-${finalAttrs.version}-aarch64-apple-darwin.tar.gz";
        hash = "sha256-RoNHwgt2TJi1rkMzrfMT+AllRDHY+bLQyXJIKGm2n68=";
      }
    else
      throw "cmakefmt: unsupported system ${stdenv.hostPlatform.system}";

  nativeBuildInputs = lib.optionals stdenv.isLinux [ autoPatchelfHook ];
  buildInputs = lib.optionals (stdenv.isLinux && stdenv.hostPlatform.system == "aarch64-linux") [
    gcc-unwrapped.lib
  ];

  unpackPhase = ''
    tar -xzf $src
    cd cmakefmt-${finalAttrs.version}-*
  '';

  installPhase = ''
    install -Dm755 cmakefmt $out/bin/cmakefmt
  '';

  meta = {
    description = "A fast, correct CMake formatter";
    longDescription = ''
      cmakefmt is a fast, correct, configurable CMake formatter written in
      Rust. It is a native-binary drop-in replacement for cmake-format with
      full legacy config conversion support.
    '';
    homepage = "https://cmakefmt.dev";
    changelog = "https://github.com/cmakefmt/cmakefmt/blob/main/CHANGELOG.md";
    license = with lib.licenses; [ mit asl20 ];
    maintainers = [ ];
    mainProgram = "cmakefmt";
    platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
  };
})
