FROM fedimint/fedimintd:v0.8.0-beta.2

ADD ./docker_entrypoint.sh /docker_entrypoint.sh
ADD ./check-web.sh /usr/local/bin/check-web.sh
RUN chmod +x /docker_entrypoint.sh /usr/local/bin/check-web.sh

COPY config_get.sh config_set.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/config_get.sh /usr/local/bin/config_set.sh

VOLUME ["/fedimintd", "/start-os"]
EXPOSE 8175/tcp

ENTRYPOINT ["/docker_entrypoint.sh"]
