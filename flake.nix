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

        memory_x = pkgs.writeTextFile {
          name = "memory.x";
          destination = "/memory.x";
          text = ''
            MEMORY {
              BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
              FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
              RAM   : ORIGIN = 0x20000000, LENGTH = 256K
            }
          '';
        };

        pico_w_firmware = builtins.fetchurl {
          url = "https://github.com/embassy-rs/embassy/raw/main/cyw43-firmware/43439A0.bin";
          sha256 = "0sy91m0anbm8r6xv6q2ba64qj8anhv2bm2jl3msc7iyvg0sf402c";
        };

        pico_w_firmware_clm = builtins.fetchurl {
          url = "https://github.com/embassy-rs/embassy/raw/main/cyw43-firmware/43439A0_clm.bin";
          sha256 = "09g2q9svqfa7ilpcyhb35iz8xaadykl61gwyrji5d9azqz2ihk0i";
        };

        pico_svd = builtins.fetchurl {
          url = "https://github.com/raspberrypi/pico-sdk/raw/master/src/rp2040/hardware_regs/rp2040.svd";
          sha256 = "1j16yqwcabrll2jgxfmbq6g3040xsf5b7971mmhcc1wgh016412s";
        };

        tools = {
          nix = with pkgs; [
            nil
            self.formatter.${system}
          ];

          rust = with pkgs;
            [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              flip-link
              probe-run
              probe-rs
              elf2uf2-rs
            ]
            ++ lib.optionals stdenv.isDarwin
            (with darwin.apple_sdk.frameworks; [Security]);
        };
      in {
        # Nix code formatter: `nix fmt`
        formatter = pkgs.alejandra;

        # Development shell: `nix develop`
        devShell = pkgs.mkShell {
          packages = tools.nix ++ tools.rust;
          MEMORY_X = memory_x;
          PICO_W_FIRMWARE = pico_w_firmware;
          PICO_W_FIRMWARE_CLM = pico_w_firmware_clm;
          PICO_SVD = pico_svd;
        };
      }
    );
}
