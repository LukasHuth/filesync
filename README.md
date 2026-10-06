# filesync

## Example .env
```
CLIENT_KEY=<client private key>
CLIENT_IP="<client ip in vpn network>"
SERVER_KEY=<server public key>
SERVER_PORT=<wireguard port>
SERVER_IP="<server ip>"
TARGET_IP="<file server ip in vpn network>"
TARGET_PORT=<file server port>
```

## get keys
add server ip to .env
run `./get_keys.sh`
