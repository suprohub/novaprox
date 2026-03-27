/disk add type=tmpfs tmpfs-max-size=16M
/interface bridge add name=containers
/interface veth add name=veth1 address=172.17.0.2/24 gateway=172.17.0.1
/interface bridge port add bridge=containers interface=veth1
/ip address add address=172.17.0.1/24 interface=containers
/ip firewall nat add chain=srcnat action=masquerade src-address=172.17.0.0/24
/container envs add list=novaprox_env key=GIT_USER value="username"
/container envs add list=novaprox_env key=GIT_EMAIL value="email@email.email"
/container envs add list=novaprox_env key=GIT_REPO value="<owner>/<repo>"
/container envs add list=novaprox_env key=GIT_TOKEN value="github_pat_<token_here>"
/container mounts add list=repo_mount src=tmp1 dst=/repo
/container add file=novaprox.tar \
    interface=veth1 \
    envlist=novaprox_env \
    mountlists=repo_mount \
    name=novaprox \
    start-on-boot=yes \
    logging=yes \
    dns=8.8.8.8 \
    user=0:0
container start novaprox