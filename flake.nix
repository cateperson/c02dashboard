{
  description = "CO₂ dashboard dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
            tailwindcss
            just
            sqlite
            pkg-config
            openssl
          ];

          RUST_BACKTRACE = "1";
          DATABASE_URL = "sqlite://./data/co2.db";
        };
      });
}
