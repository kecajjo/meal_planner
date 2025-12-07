FROM rust:trixie

USER root
    RUN useradd -m developer && \
        echo "developer:123" | chpasswd && \
        adduser developer sudo

    RUN apt-get update && apt-get install -y \
        sudo \
        git \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libasound2-dev \
        libudev-dev \
        libayatana-appindicator3-dev \
        libxdo-dev \
        libglib2.0-dev \
        default-jdk \
        unzip \
        wget && \
        rustup component add rustfmt && \
        rm -rf /var/lib/apt/lists/*

    RUN sudo apt-get update && sudo apt-get install -y binaryen && \
	rm -rf /var/lib/apt/lists/*
    

USER developer
    RUN rustup component add clippy && \
	    cargo install wasm-opt && \
        rustup target add \
	    wasm32-unknown-unknown \
    	aarch64-linux-android \
        i686-linux-android \
        armv7-linux-androideabi \
        x86_64-linux-android

    # Install cargo-binstall 
    RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

    RUN cargo binstall dioxus-cli --force

USER root
    # Install Android SDK & NDK
    RUN mkdir -p /opt/android-sdk/cmdline-tools/latest && \
        wget -O /tmp/commandlinetools.zip https://dl.google.com/android/repository/commandlinetools-linux-10406996_latest.zip && \
        unzip /tmp/commandlinetools.zip -d /opt/android-sdk/cmdline-tools/latest && \
        rm /tmp/commandlinetools.zip && \
        mv /opt/android-sdk/cmdline-tools/latest/cmdline-tools/* /opt/android-sdk/cmdline-tools/latest/ && \
        yes | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=/opt/android-sdk \
        "platform-tools" \
        "platforms;android-33" \
        "build-tools;33.0.2" \
        "ndk;25.2.9519653" \
        "emulator" \
        "system-images;android-33;google_apis;x86_64"

    RUN chown -R developer:developer /opt/android-sdk && \
        chmod -R a+rwX /opt/android-sdk

    # Install dependencies
    RUN apt-get update && apt-get install -y \
        qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils \
        libgl1-mesa-dev \
        && rm -rf /var/lib/apt/lists/*

        
USER developer
    RUN mkdir -p /home/developer/repo
    WORKDIR /home/developer/repo
    ENV PATH="/usr/local/bin:/opt/android-sdk/emulator:/opt/android-sdk/tools:/opt/android-sdk/tools/bin:/opt/android-sdk/platform-tools:/opt/android-sdk/cmdline-tools/latest/bin:$PATH"

    RUN avdmanager create avd -n mobile -k "system-images;android-33;google_apis;x86_64" --device "pixel"

USER root
# Write a readable entrypoint.sh with heredoc
RUN cat << 'EOF' >> /usr/local/bin/entrypoint.sh
#!/bin/bash
set -e

PASSWORD="123"
USER_NAME=$(whoami)

# Function to ensure user is in group by name & gid
ensure_group() {
  GROUP_NAME="$1"
  GROUP_GID="$2"
  # Skip if GID is not set
  [ -z "$GROUP_GID" ] && return
  # See if any group already owns that gid
  EXISTING_GROUP=$(getent group "$GROUP_GID" | cut -d: -f1)
  if [ -n "$EXISTING_GROUP" ] && [ "$EXISTING_GROUP" != "$GROUP_NAME" ]; then
    # Find a free gid to move the colliding group to
    NEW_GID=$(awk -F: -v tgt="$GROUP_GID" '($3>=500 && $3!=tgt){used[$3]=1} END{for(i=500;i<60000;i++) if(!used[i]){print i;exit}}' /etc/group)
    echo "$PASSWORD" | sudo -S groupmod -g "$NEW_GID" "$EXISTING_GROUP"
  fi
  # Create or fix the target group
  if getent group "$GROUP_NAME" >/dev/null; then
    CUR_GID=$(getent group "$GROUP_NAME" | cut -d: -f3)
    if [ "$CUR_GID" != "$GROUP_GID" ]; then
      echo "$PASSWORD" | sudo -S groupmod -g "$GROUP_GID" "$GROUP_NAME"
    fi
  else
    echo "$PASSWORD" | sudo -S groupadd -g "$GROUP_GID" "$GROUP_NAME"
  fi
  echo "$PASSWORD" | sudo -S usermod -aG "$GROUP_NAME" "$USER_NAME"
}

# Add user to necessary host/device groups by GID
ensure_group kvm "$KVM_GID"
ensure_group plugdev "$PLUGDEV_GID"
ensure_group usb "$USB_GID"
ensure_group video "$VIDEO_GID"

sudo -u developer /bin/bash --login
EOF

RUN chmod +x /usr/local/bin/entrypoint.sh

# Set persistent env vars for login shells and system-wide for 'developer' user

RUN echo 'ANDROID_HOME=/opt/android-sdk\n\
ANDROID_NDK_HOME=/opt/android-sdk/ndk/25.2.9519653\n\
JAVA_HOME=/usr/lib/jvm/default-java\n\
PATH="/usr/local/bin:/opt/android-sdk/emulator:/opt/android-sdk/tools:/opt/android-sdk/tools/bin:/opt/android-sdk/platform-tools:/opt/android-sdk/cmdline-tools/latest/bin:$PATH"' >> /etc/environment

RUN cat << EOF >> /home/developer/.profile
export ANDROID_HOME=/opt/android-sdk
export ANDROID_NDK_HOME=/opt/android-sdk/ndk/25.2.9519653
export JAVA_HOME=/usr/lib/jvm/default-java
export PATH="/usr/local/bin:/opt/android-sdk/emulator:/opt/android-sdk/tools:/opt/android-sdk/tools/bin:/opt/android-sdk/platform-tools:/opt/android-sdk/cmdline-tools/latest/bin:\$PATH"
EOF

RUN chown developer:developer /home/developer/.profile

USER developer
WORKDIR /home/developer/repo

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

CMD ["/bin/bash", "-c"]
