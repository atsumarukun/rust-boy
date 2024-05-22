FROM rust:1.77

ARG USERNAME=docker
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN apt-get update && apt-get install -y sudo cmake libsdl2-dev pulseaudio &&\
    groupadd -g $USER_GID $USERNAME &&\
    useradd -u $USER_UID -g $USER_GID -G sudo -m $USERNAME &&\
    echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers

USER $USERNAME

RUN rustup component add rustfmt
