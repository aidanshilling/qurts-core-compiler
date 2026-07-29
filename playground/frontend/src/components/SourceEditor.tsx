import type { ExampleScript } from "../api";

type Props = {
  source: string;
  onSourceChange: (source: string) => void;
  examples: ExampleScript[];
  onSelectExample: (name: string) => void;
};

export function SourceEditor({ source, onSourceChange, examples, onSelectExample }: Props) {
  return (
    <div className="source-editor">
      <div className="source-editor-toolbar">
        <label htmlFor="example-select">Load example:</label>
        <select
          id="example-select"
          defaultValue=""
          onChange={(event) => {
            if (event.target.value) onSelectExample(event.target.value);
          }}
        >
          <option value="" disabled>
            Select a script...
          </option>
          {examples.map((example) => (
            <option key={example.name} value={example.name}>
              {example.name}
            </option>
          ))}
        </select>
      </div>
      <textarea
        className="source-editor-textarea"
        value={source}
        onChange={(event) => onSourceChange(event.target.value)}
        spellCheck={false}
        placeholder="fn f() -> bool { true }"
      />
    </div>
  );
}
