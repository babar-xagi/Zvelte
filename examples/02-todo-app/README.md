# 📝 Example 02: Task Manager App

This example illustrates managing lists, conditional banners, and two-way form bindings in Zvelte without Virtual DOM reconciliation.

## 🚀 Running this Example

```bash
cd examples/02-todo-app
zvelte dev
```

Visit [http://127.0.0.1:3002/](http://127.0.0.1:3002/) in your browser.

## 💡 Key Concepts Demonstrated

- Typed `<state>` declarations: `todos: []string`, `completed_tasks: i32 = 1`, `new_task: string = ""`.
- Two-way form input binding: `<input ... bind:value={new_task} />`.
- Dynamic list mutation & scoped rendering: `{#each todos as todo} ... {/each}` with `click={todos.push(new_task); new_task = ""}`.
- Conditional rendering: `{#if completed_tasks >= todos.length} ... {:else} ... {/if}`.
