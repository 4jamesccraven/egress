{ pkgs, ... }:

with pkgs;
let
  manifest = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  inherit (manifest) version;

  src = ./.;

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    mkdir $out/libexec
    mv $out/bin/egressd $out/libexec

    installShellCompletion --cmd egressctl \
      --bash <(COMPLETE=bash $out/bin/egressctl) \
      --zsh <(COMPLETE=zsh $out/bin/egressctl) \
      --fish <(COMPLETE=fish $out/bin/egressctl)
  '';

  cargoLock.lockFile = ./Cargo.lock;
}
