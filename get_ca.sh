# load envs
set -a
. ./.env
set +a

ip="${SERVER_IP}"
base_location="/root/progs/filesync"
client_file_name="client"
mkdir -p ${PKI_DIR}
scp root@${ip}:${base_location}/${PKI_DIR}/ca.pem ${PKI_DIR}/ca.pem
scp root@${ip}:${base_location}/${PKI_DIR}/${client_file_name}.pem ${PKI_DIR}/${client_file_name}.pem
scp root@${ip}:${base_location}/${PKI_DIR}/${client_file_name}.key ${PKI_DIR}/${client_file_name}.key
