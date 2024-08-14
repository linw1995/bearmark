FROM alpine

RUN apk add --no-cache tzdata

COPY ./server /usr/local/bin/server
COPY ./static /app/static

EXPOSE 8000

WORKDIR /app

CMD ["server"]
