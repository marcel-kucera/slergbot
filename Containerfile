FROM rust:alpine AS build
COPY ./ /app
WORKDIR /app
RUN apk add clang
RUN cargo build --release

FROM alpine
RUN apk add openjdk21 
COPY --from=build /app/target/release/slergbot /app/
WORKDIR /app
CMD ["./slergbot"]
