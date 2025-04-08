{ pkgs ? import <nixpkgs> { } }:

let cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in pkgs.rustPlatform.buildRustPackage rec {
  pname = "cm3588-fan";
  version = cargoToml.package.version;

  src = pkgs.lib.cleanSource ./.;
  cargoLock.lockFile = "${src}/Cargo.lock";

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp target/release/${pname} $out/bin/

    # Install systemd service file
    mkdir -p $out/lib/systemd/system
    cp ${src}/${pname}.service $out/lib/systemd/system/

    runHook postInstall
  '';
}
