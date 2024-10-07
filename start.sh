#!/bin/bash
pyls --tcp --host 0.0.0.0 --port 2087 &
./github-clone-server
