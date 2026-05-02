# typhoon_lan_server

Deploys the headless TyphooN CLI LAN sync server with Docker Compose.

The role normally uses the LAN passphrase already stored by the GUI. For a fresh cache server, set `typhoon_lan_server_bootstrap_passphrase`; the CLI server will persist it into the same keyring/KV locations and reuse the saved value after that.

## Variables

```yaml
typhoon_lan_server_stack_dir: /opt/typhoon-lan-server
typhoon_lan_server_cache_host_path: /mnt/nas/typhoon-cache
typhoon_lan_server_image: typhoon-terminal:lan-server
typhoon_lan_server_lan_port: 9847
typhoon_lan_server_metrics_port: 9090
typhoon_lan_server_build_context: ""
typhoon_lan_server_bootstrap_passphrase: ""
```

Set `typhoon_lan_server_build_context` to a repo path on the target host if the role should build the image locally. Leave it empty to use `typhoon_lan_server_image`.
