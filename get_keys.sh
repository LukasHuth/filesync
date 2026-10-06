# load envs
set -a
. ./.env
set +a

ip="${SERVER_IP}"
server="ssh root@${ip} cat /etc/wireguard/server_public.key"
client_number="1"
client="ssh root@${ip} cat /etc/wireguard/client${client_number}_private.key"
echo "server public key: ${server}"
echo "client private key: ${client}"
