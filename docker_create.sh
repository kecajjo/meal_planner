#!/bin/bash
scriptDir=$(dirname $0 | xargs -i readlink -f {})
docker_registry_path="jacekmultan"
container_name="meal-planner-rust"
image_name=${docker_registry_path}/${container_name}
version="0.4.4"
does_exist=$(docker image ls $image_name:$version | grep -ci1 $container_name)
if [ $does_exist == "0" ] ; then
	docker build -t $image_name:$version $scriptDir/
fi
docker run --rm \
    --privileged \
    --name $container_name \
    -e KVM_GID=$(getent group kvm | cut -d: -f3)\
    -e PLUGDEV_GID=$(getent group plugdev | cut -d: -f3) \
    -e USB_GID=$(getent group usb | cut -d: -f3) \
    -e VIDEO_GID=$(getent group video | cut -d: -f3) \
    --net=host \
    --ipc=host \
    --shm-size=512m \
    --security-opt seccomp=unconfined \
    --security-opt apparmor=unconfined \
    --cap-add NET_ADMIN \
    --device /dev/kvm \
    --device /dev/dri \
    --device /dev/snd \
    --device /dev/video0 \
    --device /dev/input \
    --device /dev/bus/usb \
    -e DISPLAY=$DISPLAY \
    -e XDG_RUNTIME_DIR=/run/user/$(id -u) \
    -e QTWEBENGINE_DISABLE_SANDBOX=1 \
    -e QT_XCB_GL_INTEGRATION=none \
    -e LIBGL_ALWAYS_SOFTWARE=1 \
    -e XAUTHORITY=/home/developer/.Xauthority \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v "$HOME/.Xauthority:/home/developer/.Xauthority" \
    -v "$scriptDir:/home/developer/repo" \
    -it $image_name:$version
