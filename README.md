<div align="center">
<a href="https://agenticlabs.com/">
    <img src="https://raw.githubusercontent.com/agentic-labs/.github/main/assets/logo.png" alt="Agentic Labs" title="Agentic Labs" align="center" height="150px" />
</a>

# lsproxy - all your langauge servers are belong to us

<p align="center">
  <img alt="discord" src="https://img.shields.io/discord/1296271531994775552">
</p>
</div>

## Getting Started

### Run a single container
`docker run -p 8080:8080 -v $WORKSPACE_PATH:/mnt/workspace agenticlabs/lsproxy`

### Add to compose
```
services:
  lsproxy:
    image: agenticlabs/lsproxy
    ports:
      - "8080:8080"
    volumes:
      - ${WORKSPACE_PATH}:/mnt/workspace
```
