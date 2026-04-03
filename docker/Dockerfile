FROM alpine:latest AS certs
RUN apk add --no-cache ca-certificates

FROM busybox:musl

RUN adduser -D -h /home/app app

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY build_usr/xray /usr/local/bin/xray
COPY build_usr/ghcp /usr/local/bin/ghcp
COPY build_usr/novaprox /usr/local/bin/novaprox
COPY build_usr/sources/sources.txt /usr/local/sources.txt
COPY build_usr/sources/sources.txt /repo/sources.txt
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod +x /usr/local/bin/novaprox /usr/local/bin/xray /usr/local/bin/ghcp

RUN chmod +x /usr/local/bin/entrypoint.sh && \
    chown -R app:app /repo
USER app
WORKDIR /repo

CMD ["sh", "/usr/local/bin/entrypoint.sh"]