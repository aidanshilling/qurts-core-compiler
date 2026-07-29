export type FunctionResult = {
  name: string;
  ok: boolean;
  content: string;
};

export type Stage =
  | { id: string; label: string; kind: "text"; content: string }
  | { id: string; label: string; kind: "functions"; functions: FunctionResult[] };

export type CompileResponse =
  | { ok: true; stages: Stage[] }
  | { ok: false; error: string };

export type ExampleScript = {
  name: string;
  source: string;
};

export async function compile(source: string): Promise<CompileResponse> {
  const response = await fetch("/api/compile", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ source }),
  });
  return response.json();
}

export async function fetchExamples(): Promise<ExampleScript[]> {
  const response = await fetch("/api/examples");
  return response.json();
}
