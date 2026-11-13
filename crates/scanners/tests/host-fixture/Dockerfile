# Disposable test server only: never an application/production deployment recipe.
FROM ubuntu@sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c
COPY ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
RUN sed -i 's|http://|https://|g' /etc/apt/sources.list.d/ubuntu.sources \
    && apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends openssh-server iproute2 libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 2009 opentatu-inspect \
    && useradd --uid 2009 --gid 2009 --no-create-home --home-dir /var/empty/opentatu-inspect --shell /bin/sh --password '*' opentatu-inspect \
    && mkdir -p /var/empty/opentatu-inspect /run/sshd \
    && chmod 0555 /var/empty/opentatu-inspect \
    && ssh-keygen -A
COPY opentatu /usr/local/bin/opentatu
COPY authorized_key.pub /etc/ssh/opentatu_inspect_authorized_keys
COPY sshd_config /etc/ssh/sshd_config
RUN chmod 0755 /usr/local/bin/opentatu \
    && chmod 0644 /etc/ssh/opentatu_inspect_authorized_keys \
    && printf 'OPENTATU_LOG=invalid[\nOPENCODE_GO_API_KEY=phase9_fictitious_unused_key\n' > /var/empty/opentatu-inspect/.env \
    && chmod 0444 /var/empty/opentatu-inspect/.env
EXPOSE 2222
CMD ["/bin/sh", "-c", "mkdir -p /run/sshd && exec /usr/sbin/sshd -D -e"]
