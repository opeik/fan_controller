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

        rp_pico = {
          # Raspberry Pi Pico linker script.
          # See: https://docs.rust-embedded.org/cortex-m-quickstart/cortex_m_rt/index.html#memoryx
          linker_script = pkgs.writeTextFile {
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

          # Raspberry Pi Pico system view description file.
          # See: https://www.keil.com/pack/doc/CMSIS/SVD/html/index.html
          svd = builtins.fetchurl {
            url = "https://github.com/raspberrypi/pico-sdk/raw/master/src/rp2040/hardware_regs/rp2040.svd";
            sha256 = "1j16yqwcabrll2jgxfmbq6g3040xsf5b7971mmhcc1wgh016412s";
          };
        };

        rp_pico_w = {
          # Raspberry Pi Pico W firmware.
          # See: https://github.com/Infineon/wifi-host-driver/tree/master
          firmware = builtins.fetchurl {
            url = "https://github.com/Infineon/wifi-host-driver/blob/master/WiFi_Host_Driver/resources/firmware/COMPONENT_43439/43439a0.bin";
            sha256 = "15fd6hlb8j7wyya320as85cqcnkr4sjigz5n0szkdm1asgbnjdwr";
          };

          # Raspberry Pi Pico W country locale matrix.
          # See: https://github.com/Infineon/wifi-host-driver/tree/master
          clm = builtins.fetchurl {
            url = "https://github.com/Infineon/wifi-host-driver/blob/master/WiFi_Host_Driver/resources/clm/COMPONENT_43439/43439A0.clm_blob";
            sha256 = "0v43xqq3fx4ad2v4n91g9zqh6hinm3gqn1n3c1yq3nlsl4ykmyqc";
          };
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
              cargo-udeps
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
          RP_PICO_SVD = rp_pico.svd;
          RP_PICO_LINKER_SCRIPT = rp_pico.linker_script;
          RP_PICO_W_FIRMWARE = rp_pico_w.firmware;
          RP_PICO_W_CLM = rp_pico_w.clm;
        };
      }
    );
}
