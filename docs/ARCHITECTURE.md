# 🏗️ Zvelte Architecture & Internal Design

> A comprehensive deep-dive into the compiler internals, component pipeline, reactivity model, and multi-language execution runtime of the Zvelte framework.

---

## 🧭 System Overview

Zvelte is structured as a unified pipeline implemented in **Zig 0.17.1**, designed around three core principles:
1. **Deterministic Simplicity:** Scaffolds strictly two user-facing files (`app.toml` and `app.zvelte`).
2. **Fine-Grained DOM Updates:** Eliminates Virtual DOM reconciliation; reactive state mutations directly target and mutate affected DOM text cells and attributes.
3. **Multi-Language Hybrid Execution:** Coordinates Python server-side logic, Zig native compute routines, and browser JavaScript/WASM execution within one component syntax.

```mermaid
flowchart TD
    subgraph Inputs["📄 Developer Source Files"]
        TOML["app.toml (Project Configuration)"]
        SFC["app.zvelte (Single-File Component)"]
    end

    subgraph CLI["⚡ Zvelte Core CLI (Zig 0.17.1)"]
        CFG["TOML Config Parser & Validator"]
        LEX["SFC Lexer & Tokenizer"]
        PAR["Language-Neutral AST Parser"]
        SEM["Semantic Analyzer & Symbol Table"]
    end

    subgraph Adapters["🔌 Multi-Language Adapters"]
        PY_SRV["Python Server Adapter (CPython 3.14/3.15)"]
        ZIG_NAT["Zig Native Adapter (Host C ABI Shared Library)"]
        ZIG_WASM["Zig WebAssembly Adapter (wasm32-freestanding)"]
    end

    subgraph Generators["⚙️ Code Generation"]
        JS_GEN["Fine-Grained Reactive JS Codegen"]
        HTML_GEN["HTML Shell & Injected CSS Generator"]
    end

    subgraph Outputs["🚀 Runtime & Delivery Targets"]
        DEV["Embedded HTTP Dev Server (127.0.0.1:3000)"]
        DIST["dist/ Production Bundle (HTML + JS + WASM)"]
        DOM["Browser Live DOM"]
    end

    TOML --> CFG
    SFC --> LEX --> PAR
    CFG --> SEM
    PAR --> SEM

    SEM --> PY_SRV
    SEM --> ZIG_NAT
    SEM --> ZIG_WASM
    SEM --> JS_GEN
    SEM --> HTML_GEN

    JS_GEN --> DEV
    HTML_GEN --> DEV
    JS_GEN --> DIST
    HTML_GEN --> DIST
    DEV --> DOM
    DIST --> DOM
```

---

## 🧩 1. The Single-File Component (`.zvelte`) Pipeline

A `.zvelte` file consolidates server logic, native acceleration, client reactive state, markup, and styles without syntax ambiguity.

### Grammar Segmentation
The parser treats the file as distinct raw language regions:

| Region | Syntax Boundary | Parser Action | Destination |
|---|---|---|---|
| Python Server Code | `<script lang="python" context="server">` | Raw extraction up to `</script>` | Python Server Supervisor / Bridge |
| Zig Native Code | `<script lang="zig" context="native">` | Raw extraction up to `</script>` | C-ABI Native Shared Library |
| Zig WASM Code | `<script lang="zig" context="wasm">` | Raw extraction up to `</script>` | `wasm32-freestanding` Compiler |
| Reactive State | `<state>` | Line-by-line typed state parser | Reactive State Store & Dependency Graph |
| Component Template | Markup elements & text | Recursive element AST parser | Direct DOM Generation Code |
| Dynamic Expressions | `{expression}` | Expression tokenizer | Text/attribute subscription bindings |
| Event Handlers | `event={expression}` | Event attribute binding parser | DOM event listener generation |
| Styles | `<style>` | Raw CSS block extraction | Injected `<style>` head element |

```mermaid
sequenceDiagram
    autonumber
    participant Dev as Developer / File Change
    participant Lex as Lexer / Parser
    participant AST as Component AST
    participant Sem as Semantic Analyzer
    participant Gen as JS Codegen
    participant Out as Output File System

    Dev->>Lex: Parse app.zvelte
    Lex->>AST: Isolate raw script blocks & parse template
    AST->>Sem: Submit Component AST + app.toml Config
    Note over Sem: Verify mode constraints<br/>Check state variable symbols<br/>Validate event handlers
    Sem->>Gen: Emit verified semantic model
    Gen->>Out: Generate index.html & fine-grained app.js
```

---

## ⚡ 2. Fine-Grained Reactive DOM Model

Traditional frontend frameworks evaluate a Virtual DOM tree on every state update, performing structural reconciliation ($O(N)$ diffing). 

Zvelte implements **Fine-Grained Surgical Updates**:
1. During compile-time, each dynamic interpolation `{count}` is registered with its specific parent DOM node.
2. The compiler generates an atomic subscriber callback:
   ```javascript
   const txt_dyn_5 = document.createTextNode(state.count);
   subscribe(() => { txt_dyn_5.data = state.count; });
   ```
3. When an event fires (e.g., `click={count++}`):
   - The state cell is mutated: `state.count++`.
   - The scheduler flushes subscribers: `update()`.
   - Only `txt_dyn_5.data` is updated.
   - **No other DOM nodes, buttons, or parent elements are touched.**

```mermaid
flowchart LR
    subgraph EventTrigger["1. Event Trigger"]
        BTN["User clicks <button>"]
        HANDLER["addEventListener('click', () => { state.count++; update(); })"]
    end

    subgraph StateUpdate["2. State Mutation"]
        CELL["state.count = state.count + 1"]
        SCHED["update() scheduler flush"]
    end

    subgraph DirectMutation["3. Fine-Grained DOM Mutation"]
        SUB["Subscriber callback invoked"]
        NODE["txt_dyn_5.data = state.count"]
        BROWSER["Screen renders updated number"]
    end

    BTN --> HANDLER
    HANDLER --> CELL --> SCHED
    SCHED --> SUB --> NODE --> BROWSER
```

---

## 🔌 3. Multi-Language Execution Model

Zvelte provides dedicated roles for each language:

```mermaid
graph TD
    subgraph ServerSide["🖥️ Server-Side Runtime"]
        CPY["CPython Worker Process"]
        ZIG_LIB["Native Zig Shared Library (.so/.dll)"]
        CTYPES["ctypes / C ABI Bridge"]
        CPY <--> CTYPES <--> ZIG_LIB
    end

    subgraph ClientSide["🌐 Browser Client Runtime"]
        JS_RT["Zvelte JS Runtime (State + Event Scheduler)"]
        WASM_MOD["Zig WASM Module (.wasm)"]
        DOM_API["Browser DOM"]
        JS_RT <--> WASM_MOD
        JS_RT --> DOM_API
    end

    ServerSide -.->|"JSON / HTTP API / RPC"| ClientSide
```

### Script Execution Roles

- **Python (`context="server"`):**
  - Runs inside CPython on the host.
  - Used for database integrations, machine learning, external APIs, and server logic.
  - Never shipped raw to the browser.
- **Zig (`context="native"`):**
  - Compiled into high-performance host machine code.
  - Directly callable from Python via C ABI bindings.
- **Zig (`context="wasm"`):**
  - Compiled targeting `wasm32-freestanding`.
  - Loaded by the browser JavaScript runtime for heavy numerical computation or cryptographics.

---

## 📁 4. The Two-File Invariant

A central guarantee of Zvelte is the **Two-File Project Scaffold**:

```text
my-app/
├── app.toml      # The single, authoritative configuration file
└── app.zvelte    # The single, authoritative source file
```

When building or developing:
- `.zvelte/` holds disposable development artifacts and temporary cache.
- `dist/` holds production HTML and JavaScript.
- Running `zvelte clean` instantly removes `.zvelte/` and `dist/`, restoring the workspace to its exact two-file state without touching user code.
