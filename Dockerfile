# syntax=docker.io/docker/dockerfile:1

FROM gcr.io/distroless/cc-debian11:nonroot

ARG BIN_DIR
ARG TARGETARCH
COPY $BIN_DIR/$TARGETARCH/kaisantantoudaijin /usr/bin/kaisantantoudaijin
CMD ["/usr/bin/kaisantantoudaijin"]
