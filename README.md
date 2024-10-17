<div align="center">
<a href="https://agenticlabs.com/">
    <img src="https://raw.githubusercontent.com/agentic-labs/.github/main/assets/logo.png" alt="Agentic Labs" title="Agentic Labs" align="center" height="150px" />
</a>

# lsproxy - all your langauge servers are belong to us

<p align="center">
  <img alt="discord" src="https://img.shields.io/discord/1296271531994775552">
</p>
</div>

# Table of Contents

1. [What is lsproxy?](#what-is-lsproxy)
2. [Getting started](#getting-started)
3. [Examples](#examples)
4. [Contributing](#contributing)
5. [Community](#community)
6. [Supported languages](#supported-languages)

   
# <a name="what-is-lsproxy">What is lsproxy?</a>

lsproxy is a service that makes communicating with language servers outside of code editors much easier. This is a significant use case when building LLM coding agents. The major issues with language servers are:
- Each language has its own server that needs to be installed and run.
- Each server has different, poorly documented configuration settings, and it’s hard to test whether it’s working correctly.
- The interface with the language server is built to communicate with visual IDEs and mouse cursors, not text-based LLM agents.

lsproxy addresses these issues in the following ways:
- Language servers for popular languages pre-installed and automatically run depending on the files in your workspace.
- Each server is configured from project files like `tsconfig.json`, `pyproject.toml`, or `vscode/settings.json` or provides sensible defaults.
- Provides a simple API for AI agents (or humans!)

We have a good set of initial languages we support (see [Supported languages](#supported-languages)) and a simple set of API endpoints ([API spec here](https://docs.agenticlabs.com)), but we're continually adding more! If there's something you'd like to see, let us know in discord, open an issue, or make a contribution!

# <a name="getting-started">Getting started</a>
The easiest way to get started is to run it yourself! We keep the latest version up to date on docker hub, and we have a python SDK available via `pip`. If you need additional inspiration for how this can be helpful to you, check out the [Examples](#examples) section.

### Install the sdk

You can find the documentation for the SDK [here](sdk.agenticlabs.com)
```bash
pip install lsproxy-sdk
```

### Run a container or add to compose
Make sure your `WORKSPACE_PATH` is an absolute path, otherwise docker will complain.
```bash
docker run -p 5000:5000 -v $WORKSPACE_PATH:/mnt/workspace agenticlabs/lsproxy
```

```dockerfile
services:
  lsproxy:
    image: agenticlabs/lsproxy
    ports:
      - "5000:5000"
    volumes:
      - ${WORKSPACE_PATH}:/mnt/workspace
```

### Explore your workspace!

```python
from lsproxy_sdk import Lsproxy

client = Lsproxy()
file_path = "relative/path/from/language/root.cpp"
symbols = client.file_symbols(file_path)
for symbol in symbols:
    print(f"{symbol.name} is defined in {file_path}")
```

# <a name="examples">Examples</a>
Most have you spin up a `lsproxy` docker container with your own code above and then just run the python script we provide, but we provide sample code for a few examples. See the README for each one for more info.

|Name|Description|Bring your own code?|
|:-|:-|:-|
|Playground|We've implemented a sample project in different languages and a UI built with [marimo](https://github.com/marimo-team/marimo) for you to play around with|**No**|
|Code Graph|Start from a single file and visualize how your code webs out|**Yes**|

# <a name="contributing">Contributing</a>

We appreciate all contributions! You don't need to be an expert to help out.
Please see [CONTRIBUTING.md](https://github.com/agentic-labs/lsproxy/blob/main/CONTRIBUTING.md) for more details on how to get
started.

> Questions? Reach out to us [on Discord](https://discord.gg/WafeS3jN).

# <a name="community">Community</a>

We're building a community. Come hang out with us!

- 🌟 [Star us on GitHub](https://github.com/marimo-team/marimo)
- 💬 [Chat with us on Discord](https://discord.gg/WafeS3jN)
- ✏️ [Start a GitHub Discussion](https://github.com/agentic-labs/lsproxy/discussions)
- 🐦 [Follow us on Twitter](https://twitter.com/agentic_labs)
- 🕴️ [Follow us on LinkedIn](https://www.linkedin.com/company/agentic-labs)
  
# <a name="supported-languages">Supported languages</a>

We're constantly looking to add new language support or better language servers so let us know what you find!
|Language|Server|URL|
|:-|:-|:-|
|Javascript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
|Python|`pyright`|https://github.com/microsoft/pyright|
|Rust|`rust-analyzer`|https://github.com/rust-lang/rust-analyzer|
|Typescript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
