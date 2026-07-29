import { useEffect, useState } from "react";
import { compile, fetchExamples, type CompileResponse, type ExampleScript } from "./api";
import { SourceEditor } from "./components/SourceEditor";
import { StagePanel } from "./components/StagePanel";
import "./App.css";

const DEFAULT_SOURCE = `fn choose(x : bool) -> bool {\n    if x { true } else { false }\n}\n`;

function App() {
  const [source, setSource] = useState(DEFAULT_SOURCE);
  const [examples, setExamples] = useState<ExampleScript[]>([]);
  const [result, setResult] = useState<CompileResponse | null>(null);

  useEffect(() => {
    fetchExamples().then(setExamples).catch(() => setExamples([]));
  }, []);

  useEffect(() => {
    const timeout = setTimeout(() => {
      compile(source).then(setResult).catch((error) => setResult({ ok: false, error: String(error) }));
    }, 300);
    return () => clearTimeout(timeout);
  }, [source]);

  function selectExample(name: string) {
    const example = examples.find((e) => e.name === name);
    if (example) setSource(example.source);
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>qurts playground</h1>
        <p>Visualizes qurts source across each implemented lowering pass.</p>
      </header>
      <SourceEditor
        source={source}
        onSourceChange={setSource}
        examples={examples}
        onSelectExample={selectExample}
      />
      <main className="stages">
        {result === null && <p>Compiling...</p>}
        {result && !result.ok && <pre className="parse-error">{result.error}</pre>}
        {result && result.ok && result.stages.map((stage) => <StagePanel key={stage.id} stage={stage} />)}
      </main>
    </div>
  );
}

export default App;
