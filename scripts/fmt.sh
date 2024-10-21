docker build -t lsproxy-dev lsproxy
docker run --rm -v "$(pwd)/lsproxy":/usr/src/app lsproxy-dev cargo fmt
