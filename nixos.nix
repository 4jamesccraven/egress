{
  config,
  lib,
  pkgs,
  ...
}:

{
  options.services.egressd = {
    enable = lib.mkEnableOption "enable egressd";
  };

  config =
    let
      cfg = config.services.egressd;
    in
    lib.mkIf cfg.enable {
      users.users.egress = {
        isSystemUser = true;
        group = "egress";
      };
      users.groups.egress = { };

      environment.systemPackages = [ pkgs.egress ];

      systemd.services.egressd = {
        description = "Egress Surveillance Daemon";
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "simple";
          User = "egress";
          Group = "egress";

          ExecStart = "${pkgs.egress}/libexec/egressd";

          RuntimeDirectory = "egress";
          RuntimeDirectoryMode = "0755";

          StateDirectory = "egress";
          StateDirectoryMode = "0750";

          Restart = "on-failure";
          RestartSec = 5;
        };
      };
    };
}
