FROM alpine

RUN apk add --no-cache tzdata

COPY ./server /usr/local/bin/server

EXPOSE 8000

CMD ["server"]
