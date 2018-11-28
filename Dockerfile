# Multistage docker build, requires docker 17.05

# builder stage
FROM nvidia/cuda:10.0-devel as builder

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN set -ex && \
    apt-get update && \
    apt-get --no-install-recommends --yes install \
        libncurses5-dev \
        libncursesw5-dev \
        cmake \
        git \
        curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN git clone https://github.com/mimblewimble/grin-miner && cd grin-miner && git submodule update --init

RUN cd grin-miner && sed -i '/cuckoo-miner" }/s/^/#/' Cargo.toml && sed -i '/^#.*build-cuda-plugins"]/s/^#//' Cargo.toml

RUN cd grin-miner && $HOME/.cargo/bin/cargo build --release

# runtime stage
FROM nvidia/cuda:10.0-base

RUN set -ex && \
    apt-get update && \
    apt-get --no-install-recommends --yes install \
    libncurses5 \
    libncursesw5

COPY --from=builder /grin-miner/target/release/grin-miner /grin-miner/target/release/grin-miner
COPY --from=builder /grin-miner/target/release/plugins/* /grin-miner/target/release/plugins/
COPY --from=builder /grin-miner/grin-miner.toml /grin-miner/grin-miner.toml

WORKDIR /grin-miner

RUN sed -i -e 's/run_tui = true/run_tui = false/' grin-miner.toml

RUN echo $'#!/bin/bash\n\
if [ $# -eq 1 ]\n\
   then\n\
sed -i -e 's/127.0.0.1/\$1/g' grin-miner.toml\n\
fi\n\
./target/release/grin-miner' > run.sh

# If the grin server is not at 127.0.0.1 provide the ip or hostname to the container
# by command line (i.e. docker run --name miner1 --rm -i -t miner_image 1.2.3.4)

ENTRYPOINT ["sh", "run.sh"]