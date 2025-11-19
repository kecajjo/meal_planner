FROM rust:trixie

USER root
    RUN useradd -m developer && \
        echo "developer:123" | chpasswd && \
        adduser developer sudo

    RUN apt-get update && apt-get install -y \
        sudo \
        git && \
        rustup component add rustfmt && \
        rm -rf /var/lib/apt/lists/*


USER developer
    RUN mkdir -p /home/developer/repo
    WORKDIR /home/developer/repo

ENTRYPOINT ["/bin/bash", "-c"]
