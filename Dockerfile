FROM alpine

RUN apk add --no-cache tzdata

COPY ./server /usr/local/bin/server
COPY ./static /app/static

EXPOSE 8000
ENV BM_UI_PATH = ./static
ENV BM_ADDRESS = 0.0.0.0

WORKDIR /app

CMD ["server"]
