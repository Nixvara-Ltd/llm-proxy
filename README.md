# LLM Proxy Harness

A local, cross-platform proxy server and Ratatui-based TUI dashboard for monitoring, routing, and cost-capping autonomous AI agents.

## Overview
This project acts as a local proxy that natively intercepts OpenAI-formatted API requests. It allows you to trick autonomous coding agents (like Aider, Claude Code, Cline, or AutoGPT) into using alternative models (like Google Gemini, Anthropic Claude, or Groq) without changing their source code.

While the proxy runs in the background, a beautiful Terminal UI (TUI) provides a live dashboard to monitor the agent's exact behavior.

## Features
- **OpenAI Compatible Proxy**: Send standard OpenAI payloads to `localhost:8080` and they are automatically translated and routed to the correct provider using the unified `genai` crate.
- **Cost-Cap Kill Switch**: Sets a hard daily financial limit tracked in a local SQLite database. If an agent goes rogue and burns through your budget, the proxy immediately severs the connection and returns a `429 Too Many Requests`.
- **Prompt Inspect Mode**: Agents often hide their system prompts. This proxy intercepts and displays the exact prompts being sent to the LLM in real-time on your dashboard.
- **Cross-Platform**: Built in Rust, this runs natively on Mac, Linux, and Windows.

---

## Setup & Execution

### Prerequisites
- **Rust**: Ensure you have Rust installed via `rustup`.
- **C/C++ Build Tools**: Required for compiling the local SQLite database driver (`rusqlite`).
  - **Mac**: Automatically installed with Xcode Command Line Tools.
  - **Windows**: Install the standard Visual Studio C++ Build Tools (usually bundled with the Rust MSVC installer).
  - **Linux**: Install `build-essential` or `gcc`.

### Configuration
API keys are injected directly from your environment variables. Ensure the following are set in your terminal profile or a local `.env` file before running:
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="AIza..."
export GROQ_API_KEY="gsk_..."
```

### Running the Proxy
Open a terminal in the project directory and run:
```bash
cargo run
```
This will start the Axum proxy on `http://127.0.0.1:8080/v1` and take over your terminal with the Ratatui dashboard.

*(Note: Tracing logs are automatically redirected to `proxy.log` so they do not corrupt the TUI rendering).*

---

## Usage: Pointing your Agents
To use the proxy, override the default OpenAI Base URL in your agent or script to point to `localhost`. 

Because you are bypassing actual OpenAI, you will usually need to supply a dummy API key to bypass local validation checks in most tools.

**Example: Aider**
```bash
export OPENAI_BASE_URL="http://127.0.0.1:8080/v1"
export OPENAI_API_KEY="dummy"

aider --model openai/gemini-2.0-flash
```

**Example: Python Scripts**
```python
import openai

client = openai.OpenAI(
    base_url="http://127.0.0.1:8080/v1",
    api_key="dummy"
)

response = client.chat.completions.create(
    model="claude-3-5-sonnet-20240620",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

**Example: VS Code (Continue.dev / Cline)**
1. Open the extension's model settings.
2. Add a new **OpenAI Compatible** provider.
3. Set the **Base URL** to `http://127.0.0.1:8080/v1`.
4. Enter any random string for the API Key.
5. Set the model to your desired underlying model (e.g., `gemini-2.0-flash`).
