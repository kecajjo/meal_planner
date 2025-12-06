#!/bin/bash
scriptDir=$(dirname $0 | xargs -i readlink -f {})
docker_registry_path="jacekmultan"
container_name="meal-planner-rust"
image_name=${docker_registry_path}/${container_name}
version="0.3.1"
does_exist=$(docker image ls $image_name:$version | grep -ci1 $container_name)
if [ $does_exist == "0" ] ; then
	docker build -t $image_name:$version $scriptDir/
fi
docker run --rm \
    --privileged \
    --name $container_name \
    --env DISPLAY \
    --network host \
    -v $HOME/.Xauthority:/root/.Xauthority \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v "$scriptDir:/home/developer/repo" \
    -it $image_name:$version /bin/bash
