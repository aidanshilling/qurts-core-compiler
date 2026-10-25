use lower::lower_program;
use melior::{
    Context,
    ir::{
        Module,
        block::BlockLike,
        operation::{OperationLike, OperationRef},
    },
};
use transforms::{CONTROLLED_CONJUGATION, INFER_DROPS, MQT_OPTIMIZE, TO_QCO, gate_summary, reparse, run_pipeline};
use parser::{QurtsParser, Rule};
use pest::Parser;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Serialize)]
pub struct FunctionResult {
    pub name: String,
    pub ok: bool,
    pub content: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageContent {
    Text { content: String },
    Functions { functions: Vec<FunctionResult> },
}

#[derive(Serialize)]
pub struct Stage {
    pub id: String,
    pub label: String,
    #[serde(flatten)]
    pub content: StageContent,
}

fn text_stage(id: &str, label: &str, content: String) -> Stage {
    Stage { id: id.to_string(), label: label.to_string(), content: StageContent::Text { content } }
}

fn function_names_in_order(source: &str) -> Vec<String> {
    QurtsParser::parse(Rule::program, source)
        .expect("source already validated to parse")
        .filter(|pair| pair.as_rule() == Rule::function)
        .map(|pair| pair.into_inner().next().expect("function has a name").as_str().to_string())
        .collect()
}

/// Parses `source` and runs it through every lowering stage currently implemented,
/// returning a JSON-serializable result: `{"ok": false, "error": ...}` on a parse
/// failure, or `{"ok": true, "stages": [...]}` — one entry per pipeline stage.
/// Adding a later pass (qduc/qauc lowering) means appending another `Stage` here;
/// nothing about this shape or the frontend that renders it needs to change.
pub fn compile_source(source: &str) -> Value {
    if let Err(error) = QurtsParser::parse(Rule::program, source) {
        return json!({ "ok": false, "error": error.to_string() });
    }

    let source_stage = text_stage("source", "Source", source.to_string());

    let cst_pairs = QurtsParser::parse(Rule::program, source).expect("checked above");
    let cst_stage =
        text_stage("cst", "Parse Tree (CST)", parser::format_parse_tree(cst_pairs));

    let context = transforms::context();
    let pairs = QurtsParser::parse(Rule::program, source).expect("checked above");
    let result = lower_program(&context, pairs);

    let failed: HashSet<&str> = result.errors.iter().map(|(name, _)| name.as_str()).collect();
    let error_text: std::collections::HashMap<&str, String> =
        result.errors.iter().map(|(name, error)| (name.as_str(), error.to_string())).collect();

    let mut successful_ops: Vec<OperationRef> = Vec::new();
    let mut next = result.module.body().first_operation();
    while let Some(op) = next {
        next = op.next_in_block();
        successful_ops.push(op);
    }
    let mut successful_ops = successful_ops.into_iter();

    let functions = function_names_in_order(source)
        .into_iter()
        .map(|name| {
            if failed.contains(name.as_str()) {
                FunctionResult { ok: false, content: error_text[name.as_str()].clone(), name }
            } else {
                let op = successful_ops
                    .next()
                    .expect("one lowered op per successful function, in source order");
                FunctionResult { ok: true, content: op.to_string(), name }
            }
        })
        .collect();

    let pass1_stage = Stage {
        id: "pass1".to_string(),
        label: "Pass 1: Plain MLIR".to_string(),
        content: StageContent::Functions { functions },
    };

    let mut stages = vec![source_stage, cst_stage, pass1_stage];
    let mut module = result.module;
    'pipeline: {
        for (id, label, pipeline) in [
            ("drops", "Pass 2: Drop inference (qauc.uncompute)", INFER_DROPS),
            ("qco", "Pass 3: MQT qco (naive qif lowering)", TO_QCO),
        ] {
            let (stage, ok) = pipeline_stage(&context, &mut module, id, label, pipeline);
            stages.push(stage);
            if !ok {
                break 'pipeline;
            }
        }
        let mut naive = reparse(&context, &module);
        let (stage, _) =
            pipeline_stage(&context, &mut naive, "mqt-naive", "Pass 4a: naive + mqt-qco-default", MQT_OPTIMIZE);
        stages.push(stage);
        let (stage, ok) = pipeline_stage(
            &context,
            &mut module,
            "conjugation",
            "Pass 4b: Controlled conjugation",
            CONTROLLED_CONJUGATION,
        );
        stages.push(stage);
        if !ok {
            break 'pipeline;
        }
        let (stage, _) = pipeline_stage(
            &context,
            &mut module,
            "mqt-conjugation",
            "Pass 5b: conjugation + mqt-qco-default",
            MQT_OPTIMIZE,
        );
        stages.push(stage);
        stages.push(text_stage(
            "summary",
            "Gate counts",
            format!(
                "naive + mqt-qco-default:\n{}\n\ncontrolled-conjugation + mqt-qco-default:\n{}",
                gate_summary(&naive),
                gate_summary(&module)
            ),
        ));
    }

    json!({ "ok": true, "stages": stages })
}

fn pipeline_stage(context: &Context, module: &mut Module, id: &str, label: &str, pipeline: &str) -> (Stage, bool) {
    match run_pipeline(context, module, pipeline) {
        Ok(()) => (text_stage(id, label, module.as_operation().to_string()), true),
        Err(error) => (text_stage(id, label, format!("error: {error}")), false),
    }
}
