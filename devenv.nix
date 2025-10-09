{pkgs, ...}: {
  packages = [pkgs.figlet pkgs.lolcat];

  languages.rust = {
    channel = "stable";
    enable = true;
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };

  enterShell = ''
    echo DRSM | figlet -c -f slant | lolcat -a
  '';

  git-hooks.hooks = {
    clippy = {
      enable = true;
      settings.allFeatures = true;
      settings.extraArgs = "--all-targets -- -D warnings -D clippy::pedantic";
    };
    rustfmt = {
      enable = true;
      settings.all = true;
    };
    unit-tests = {
      enable = true;
      name = "Unit Tests";
      entry = "cargo test";
      files = "\\.rs$";
      pass_filenames = false;
    };
  };
}
