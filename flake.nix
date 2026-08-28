{
  description = "";

  inputs.nixpkgs.url = "https://nixos.org/channels/nixpkgs-unstable/nixexprs.tar.xz";

  outputs =
    { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      eachDefaultSystem =
        function:
        lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (system: function nixpkgs.legacyPackages.${system});
    in
    {
      devShells = eachDefaultSystem (pkgs: {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            libgcc
            rustc
            clippy

            socat
            sqlite
            just
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      });

      packages = eachDefaultSystem (pkgs: {
        default = pkgs.callPackage ./package.nix { };
      });

      overlays.default = _final: prev: {
        egress = prev.callPackage ./package.nix { };
      };

      nixosModules.default = import ./nixos.nix;
    };
}
