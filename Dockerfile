# syntax=docker.io/docker/dockerfile:1

ARG BASE_IMAGE
FROM $BASE_IMAGE

ARG BIN_DIR
ARG TARGETARCH
COPY $BIN_DIR/$TARGETARCH/kaisantantoudaijin /usr/bin/kaisantantoudaijin
CMD ["/usr/bin/kaisantantoudaijin"]
