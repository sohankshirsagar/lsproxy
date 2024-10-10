#!/bin/bash

docker build -t lsp-box-builder -f dockerfiles/build .
docker build -t lsp-box-runner -f dockerfiles/run .

docker run -v `pwd`:/usr/src/app lsp-box-builder
docker run -p 8080:8080 -v `pwd`/target/debug:/usr/src/app/bin lsp-box-runner
