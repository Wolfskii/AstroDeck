FROM node:20-bookworm

RUN apt-get update && apt-get install -y \
  build-essential \
  curl \
  file \
  git \
  libayatana-appindicator3-dev \
  libglib2.0-dev \
  libgtk-3-dev \
  libsoup-3.0-dev \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  librsvg2-dev \
  patchelf \
  pkg-config \
  wget \
  xdg-utils \
  xz-utils \
  zip \
  && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace
