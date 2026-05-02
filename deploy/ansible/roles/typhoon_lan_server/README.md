# typhoon_lan_server

Deploys the headless TyphooN CLI LAN sync server with Docker Compose.

The role does not manage the LAN passphrase. Configure it once in the GUI LAN Sync panel; the CLI server reads the same `lan_sync_passphrase` from the OS keyring when available and from the mounted cache KV key `cred:lan_sync_passphrase`.

## Variables

```yaml
typhoon_lan_server_stack_dir: /opt/typhoon-lan-server
typhoon_lan_server_cache_host_path: /mnt/nas/typhoon-cache
typhoon_lan_server_image: typhoon-terminal:lan-server
typhoon_lan_server_lan_port: 9847
typhoon_lan_server_metrics_port: 9090
typhoon_lan_server_build_context: ""
```

Set `typhoon_lan_server_build_context` to a repo path on the target host if the role should build the image locally. Leave it empty to use `typhoon_lan_server_image`.
