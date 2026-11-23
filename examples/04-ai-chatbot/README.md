# ⚡ Example 04: AI Chatbot (Google Gemini & OpenAI GPT-4o)

A complete AI-native single-file chatbot built with **Zvelte** and **Python**:
- **Python Server RPC:** Live LLM inference using Google Gemini (`gemini-2.5-flash`) and OpenAI (`gpt-4o-mini`).
- **Zero Secrets in Client Assets:** API keys (`GEMINI_API_KEY`, `OPENAI_API_KEY`) stay strictly inside the CPython worker process.
- **Zero pip Dependencies Required:** Uses standard library `urllib.request` out-of-the-box, or official Python SDKs (`google-genai`, `openai`).
- **Zero-Config Simulation Fallback:** If no API key is exported, it automatically provides an intelligent mock simulation so the app runs immediately.

---

## 🚀 Running this Example

```bash
cd examples/04-ai-chatbot

# Optional: Export your API key for live LLM inference
export GEMINI_API_KEY="your-gemini-api-key"
# or
export OPENAI_API_KEY="your-openai-api-key"

# Run development server with live Python worker supervisor
zvelte dev
```

Visit [http://127.0.0.1:3004/](http://127.0.0.1:3004/) in your browser.
