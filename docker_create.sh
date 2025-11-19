#!/bin/bash
scriptDir=$(dirname $0 | xargs -i readlink -f {})
container_name="meal_planner"
version="0.1"
does_exist=$(docker image ls $container_name:$version | grep -ci1 $container_name)
if [ $does_exist == "0" ] ; then
	docker build -t $container_name:$version $scriptDir/
fi
docker run --rm \
    --privileged \
    --name $container_name \
    --env DISPLAY \
    --network host \
    -v $HOME/.Xauthority:/root/.Xauthority \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v "$scriptDir:/home/developer/repo" \
    -it $container_name:$version /bin/bash
