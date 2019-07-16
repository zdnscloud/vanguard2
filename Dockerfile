FROM rust:1.36.0 as build

# create a new empty shell project
#RUN USER=root cargo new --bin vanguard2
WORKDIR /vanguard2

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./auth ./auth
COPY ./datasrc ./datasrc

# this build step will cache your dependencies
RUN cargo build --release

# our final base
FROM pingcap/alpine-glibc

# copy the build artifact from the build stage
COPY --from=build /vanguard2/target/release/auth /auth

EXPOSE 53/udp
# set the startup command to run your binary
ENTRYPOINT ["/auth"]
