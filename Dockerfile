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
        rmdir /opt/android-sdk/cmdline-tools/latest/cmdline-tools && \
        yes | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=/opt/android-sdk \
        "platform-tools" \
        "platforms;android-33" \
        "build-tools;33.0.2" \
        "ndk;25.2.9519653" \
        "emulator" \
        "system-images;android-33;google_apis;x86_64"
        
    # Set JAVA_HOME and ensure /usr/local/bin is in PATH for all shells
    ENV ANDROID_HOME=/opt/android-sdk
    ENV ANDROID_NDK_HOME=/opt/android-sdk/ndk/25.2.9519653
    ENV JAVA_HOME=/usr/lib/jvm/default-java
    ENV PATH="/usr/local/bin:$PATH"


USER developer
    RUN mkdir -p /home/developer/repo
    WORKDIR /home/developer/repo

ENTRYPOINT ["/bin/bash", "-c"]
