{
  description = "rtk - Rust Token Killer (local patched fork)";

  inputs = {
    # rtk requires Rust >= 1.91, so track unstable for a recent toolchain.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      rtk = pkgs.rustPlatform.buildRustPackage {
        pname = "rtk";
        version = "0.42.3";

        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        # 15 tests touch $HOME / PATH / create hook artifacts and fail in the
        # network-less, restricted Nix sandbox (Permission denied). They pass
        # in a normal env; skip the in-build test phase. Run `cargo test`
        # inside `nix develop` if you want the full suite.
        doCheck = false;

        # rusqlite (bundled SQLite) and ring (rustls) compile vendored C — the
        # stdenv C compiler covers both. No system sqlite/openssl needed:
        # networking is ureq -> rustls, sqlite is statically bundled.
        nativeBuildInputs = [ pkgs.pkg-config ];

        meta = with pkgs.lib; {
          description = "High-performance CLI proxy that minimizes LLM token consumption";
          homepage = "https://github.com/mtszsobczak/rtk";
          license = licenses.asl20;
          mainProgram = "rtk";
        };
      };
    in
    {
      packages.${system} = {
        default = rtk;
        inherit rtk;
      };

      devShells.${system}.default = pkgs.mkShell {
        # Iterative dev: `nix develop` then `cargo build --release`.
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer
          gcc
          pkg-config
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    };
}
