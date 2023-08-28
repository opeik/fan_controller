{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        tools = {
          nix = with pkgs; [
            nil
            self.formatter.${system}
          ];

          rust = with pkgs; [
            (rust-bin.selectLatestNightlyWith (toolchain:
              toolchain.default.override {
                extensions = ["rust-src"];
                targets = ["thumbv6m-none-eabi"];
              }))
            flip-link
            probe-run
            probe-rs
            elf2uf2-rs
          ];
        };
      in {
        # Nix code formatter: `nix fmt`
        formatter = pkgs.alejandra;

        # Development shell: `nix develop`
        devShell = pkgs.mkShell {
          packages = tools.nix ++ tools.rust;
        };
      }
    );
}
