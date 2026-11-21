# ⚡ Example 01: Reactive Counter

This example demonstrates fine-grained reactive state updates in Zvelte using a single `.zvelte` file.

## 🚀 Running this Example

```bash
cd examples/01-counter
zvelte dev
```

Then navigate to [http://127.0.0.1:3001/](http://127.0.0.1:3001/) in your browser.

## 💡 Key Concepts Demonstrated

- Declaring a client state variable: `count: i32 = 0` inside `<state>`.
- Binding dynamic state to text: `<span class="value">{count}</span>`.
- Direct event expressions: `click={count++}`, `click={count--}`, and `click={count = 0}`.
- Zero Virtual DOM overhead: updates only the specific text node holding `{count}`.
