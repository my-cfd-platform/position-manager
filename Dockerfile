FROM rust:slim
COPY ./target/release/position-manager ./target/release/position-manager

ARG app_version
ARG app_compilation_date
ENV APP_VERSION=${app_version}
ENV APP_COMPILATION_DATE=${app_compilation_date}

ENTRYPOINT ["./target/release/position-manager"]