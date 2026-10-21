import "./App.css";

export default function App() {
  return (
    <main className="app">
      <p className="status">Task 001 · Foundation</p>
      <h1>Slotshot</h1>
      <p>A lightweight screenshot utility for macOS and Windows.</p>
      <section aria-labelledby="status-heading">
        <h2 id="status-heading">The foundation is ready.</h2>
        <p>
          This is a development placeholder built with Tauri, React, and
          TypeScript. Screenshot tools are not available yet.
        </p>
      </section>
      <p className="note">
        Planned: region capture, a visual screenshot clipboard, and configurable
        shortcuts.
      </p>
    </main>
  );
}
