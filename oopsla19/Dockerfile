FROM debian:latest

MAINTAINER Ranadeep Biswas "ranadeep@irif.fr"

RUN apt update -y
RUN apt install -y build-essential git curl minisat
RUN apt install -y python3 python3-psutil python3-numpy python3-matplotlib 
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN git clone --single-branch --branch oopsla19 https://gitlab.math.univ-paris-diderot.fr/ranadeep/dbcop.git /root/dbcop
RUN cargo install --path /root/dbcop
RUN cargo install --path /root/dbcop --example history_duration
WORKDIR /root/dbcop/oopsla19
RUN tar -xf executions.tar.bz
RUN rm executions.tar.bz
