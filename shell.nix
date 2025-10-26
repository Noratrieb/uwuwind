{ pkgs ? import <nixpkgs> { } }: pkgs.mkShell {
  buildInputs = with pkgs; [
    gcc
    clang_21
    llvmPackages_21.bintools
  ];
  packages = (with pkgs; [
    gef
    rust-bindgen
    shellcheck
  ]);
}
