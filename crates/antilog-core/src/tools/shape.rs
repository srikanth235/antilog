//! Helps an agent call `seal` and `measure` right: a complete example, and for a bad input the
//! path of the failing field with a one-line example of its shape.

use serde::de::DeserializeOwned;
use serde_json::Value;

pub const SEAL_DESCRIPTION: &str = concat!(
    "Propose the experiment's definition (metric, evaluation, pinned commits, datasets, repeats). Runs the seal checks and puts a Seal experiment card in front of a person. Nothing is measured until it is sealed. ",
    "Field names are camelCase and unknown fields are refused. Required: definition (metric {name, unit, direction: \"minimize\"|\"maximize\"}, kind \"speed\", evaluation {command, repos: [{repo, commit}], protected: [{repo, glob}]}, guardrails [] , data {evaluation: {id, version}, holdout: {id, version}}, repeats >= 2 (usually 5), quickCheck (usually true), patience >= 1 (usually 6), holdoutCap >= 1 (usually 20), valueModel null, criteria 1). ",
    "files lists the protected files with their sha256; cheats plants all four standard cheats (skip the work, hard-code the answer, return early, behave differently on the holdout), each {name, commits: [{repo, sha}], changed: [[repo, path]]}. ",
    "A protected path is an object, not a string; a commit is a full sha, pinned per repo. A speed example that parses: ",
    r#"{"definition":{"metric":{"name":"seconds","unit":"s","direction":"minimize"},"kind":"speed","evaluation":{"command":"node bench.js","repos":[{"repo":"app","commit":"68ed4c1a0b7e5f3d9c2a41b6e8f07d35a9c1e2b4"}],"protected":[{"repo":"app","glob":"bench.js"}]},"guardrails":[],"data":{"evaluation":{"id":"bench-eval","version":1},"holdout":{"id":"bench-holdout","version":1}},"repeats":5,"quickCheck":true,"patience":6,"holdoutCap":20,"valueModel":null,"criteria":1},"files":[{"repo":"app","path":"bench.js","sha256":"<sha256 hex>"}],"cheats":[{"name":"skip the work","commits":[{"repo":"app","sha":"<sha>"}],"changed":[["app","src/main.js"]]},{"name":"hard-code the answer","commits":[{"repo":"app","sha":"<sha>"}],"changed":[["app","src/main.js"]]},{"name":"return early","commits":[{"repo":"app","sha":"<sha>"}],"changed":[["app","src/main.js"]]},{"name":"behave differently on the holdout","commits":[{"repo":"app","sha":"<sha>"}],"changed":[["app","src/main.js"]]}]}"#
);

pub const MEASURE_DESCRIPTION: &str = concat!(
    "Measure a change. Returns a ticket at once; the judge's outcome arrives later as a message. Give one commit per repo changed ({repo, sha}, a full sha) and the Kept trial it started from (parent: a trial id from history; omit it for the baseline). Required: commits, hypothesis. Example: ",
    r#"{"commits":[{"repo":"app","sha":"<full commit sha>"}],"hypothesis":"cache the parsed config"}"#
);

/// One node of the shape of an input, to find the field that does not parse.
enum Node {
    Leaf(fn(&Value) -> bool),
    Obj(&'static [(&'static str, bool, Node)]),
    Arr(&'static Node),
}

fn ok<T: DeserializeOwned>(v: &Value) -> bool {
    T::deserialize(v).is_ok()
}

use antilog_types::judge::{Direction, EvaluationKind, ValueModelRef};

const STR: Node = Node::Leaf(ok::<String>);
const UINT: Node = Node::Leaf(ok::<u32>);
const FLAG: Node = Node::Leaf(ok::<bool>);
const SHA: Node = Node::Obj(&[("repo", true, STR), ("sha", true, STR)]);
const DATASET: Node = Node::Obj(&[("id", true, STR), ("version", true, UINT)]);

const DEFINITION: Node = Node::Obj(&[
    (
        "metric",
        true,
        Node::Obj(&[
            ("name", true, STR),
            ("unit", true, STR),
            ("direction", true, Node::Leaf(ok::<Direction>)),
        ]),
    ),
    ("kind", true, Node::Leaf(ok::<EvaluationKind>)),
    (
        "evaluation",
        true,
        Node::Obj(&[
            ("command", true, STR),
            (
                "repos",
                true,
                Node::Arr(&Node::Obj(&[("repo", true, STR), ("commit", true, STR)])),
            ),
            (
                "protected",
                true,
                Node::Arr(&Node::Obj(&[("repo", true, STR), ("glob", true, STR)])),
            ),
        ]),
    ),
    (
        "guardrails",
        true,
        Node::Arr(&Node::Obj(&[
            ("name", true, STR),
            ("direction", true, Node::Leaf(ok::<Direction>)),
            ("tolerance", true, Node::Leaf(ok::<f64>)),
        ])),
    ),
    (
        "data",
        true,
        Node::Obj(&[("evaluation", true, DATASET), ("holdout", true, DATASET)]),
    ),
    ("repeats", true, UINT),
    ("quickCheck", true, FLAG),
    ("patience", true, UINT),
    ("holdoutCap", true, UINT),
    ("valueModel", false, Node::Leaf(ok::<Option<ValueModelRef>>)),
    ("criteria", true, UINT),
]);

const SEAL: Node = Node::Obj(&[
    ("definition", true, DEFINITION),
    (
        "files",
        false,
        Node::Arr(&Node::Obj(&[
            ("repo", true, STR),
            ("path", true, STR),
            ("sha256", true, STR),
        ])),
    ),
    (
        "cheats",
        false,
        Node::Arr(&Node::Obj(&[
            ("name", true, STR),
            ("commits", true, Node::Arr(&SHA)),
            (
                "changed",
                false,
                Node::Arr(&Node::Leaf(ok::<(String, String)>)),
            ),
        ])),
    ),
]);

const MEASURE: Node = Node::Obj(&[
    ("commits", true, Node::Arr(&SHA)),
    ("hypothesis", true, STR),
    ("parent", false, Node::Leaf(ok::<Option<String>>)),
    ("version", false, Node::Leaf(ok::<Option<u32>>)),
]);

fn walk(node: &Node, v: &Value, path: &str) -> Option<String> {
    match node {
        Node::Leaf(check) => (!check(v)).then(|| path.to_owned()),
        Node::Arr(item) => match v.as_array() {
            None => Some(path.to_owned()),
            Some(items) => items
                .iter()
                .enumerate()
                .find_map(|(i, x)| walk(item, x, &format!("{path}[{i}]"))),
        },
        Node::Obj(fields) => {
            let Some(map) = v.as_object() else {
                return Some(path.to_owned());
            };
            let dot = |key: &str| {
                if path.is_empty() {
                    key.to_owned()
                } else {
                    format!("{path}.{key}")
                }
            };
            if let Some(key) = map.keys().find(|k| !fields.iter().any(|(n, ..)| n == k)) {
                return Some(dot(key));
            }
            if fields
                .iter()
                .any(|(n, required, _)| *required && !map.contains_key(*n))
            {
                return Some(path.to_owned());
            }
            fields
                .iter()
                .find_map(|(n, _, child)| map.get(*n).and_then(|x| walk(child, x, &dot(n))))
        }
    }
}

/// The path of the first field that does not parse, such as `definition.evaluation.protected[0]`.
pub fn locate(tool: &str, value: &Value) -> Option<String> {
    let node = match tool {
        "seal" => &SEAL,
        "measure" => &MEASURE,
        _ => return None,
    };
    walk(node, value, "")
}

/// A one-line example of the shape at `path`.
pub fn example(path: &str) -> Option<&'static str> {
    let mut plain = String::new();
    let mut skip = false;
    for c in path.chars() {
        match c {
            '[' => skip = true,
            ']' => skip = false,
            _ if !skip => plain.push(c),
            _ => {}
        }
    }
    let last = plain.rsplit('.').next().unwrap_or("");
    Some(match plain.as_str() {
        "definition" => {
            "{\"metric\":{...},\"kind\":\"speed\",\"evaluation\":{...},\"guardrails\":[],\"data\":{...},\"repeats\":5,\"quickCheck\":true,\"patience\":6,\"holdoutCap\":20,\"valueModel\":null,\"criteria\":1}"
        }
        "definition.metric" => "{\"name\":\"seconds\",\"unit\":\"s\",\"direction\":\"minimize\"}",
        "definition.metric.direction" | "definition.guardrails.direction" => {
            "\"minimize\" (lower is better) or \"maximize\" (higher is better)"
        }
        "definition.metric.unit" => "\"s\" (a short unit string such as \"s\", \"ms\", \"MB\")",
        "definition.kind" => "\"speed\" (the only kind)",
        "definition.evaluation" => {
            "{\"command\":\"node bench.js\",\"repos\":[{\"repo\":\"app\",\"commit\":\"<full sha>\"}],\"protected\":[{\"repo\":\"app\",\"glob\":\"bench.js\"}]}"
        }
        "definition.evaluation.command" => {
            "\"node bench.js\" (the shell command; its last output line is the measurement)"
        }
        "definition.evaluation.repos" => {
            "[{\"repo\":\"app\",\"commit\":\"<full sha>\"}] (one pin per repo)"
        }
        "definition.evaluation.protected" => {
            "[{\"repo\":\"app\",\"glob\":\"bench.js\"}] (objects, not strings; glob may be bench/**)"
        }
        "definition.guardrails" => {
            "[{\"name\":\"memory_mb\",\"direction\":\"minimize\",\"tolerance\":5}] or []"
        }
        "definition.data" => {
            "{\"evaluation\":{\"id\":\"bench-eval\",\"version\":1},\"holdout\":{\"id\":\"bench-holdout\",\"version\":1}}"
        }
        "definition.data.evaluation" | "definition.data.holdout" => {
            "{\"id\":\"bench-eval\",\"version\":1} (id a string, version an integer)"
        }
        "definition.repeats" => "5 (an integer, at least 2)",
        "definition.quickCheck" => "true (a boolean)",
        "definition.patience" => "6 (an integer, at least 1)",
        "definition.holdoutCap" => "20 (an integer, at least 1)",
        "definition.valueModel" => {
            "null (or {\"id\":\"<value model id>\",\"version\":1} for the project's model)"
        }
        "definition.criteria" => "1 (the acceptance criteria version)",
        "files" => "[{\"repo\":\"app\",\"path\":\"bench.js\",\"sha256\":\"<64 hex>\"}]",
        "cheats" => {
            "[{\"name\":\"skip the work\",\"commits\":[{\"repo\":\"app\",\"sha\":\"<sha>\"}],\"changed\":[[\"app\",\"src/main.js\"]]}, ...] (all four standard cheats)"
        }
        "cheats.commits" | "commits" => {
            "[{\"repo\":\"app\",\"sha\":\"<full commit sha>\"}] (objects, not strings)"
        }
        "cheats.changed" => "[[\"app\",\"src/main.js\"]] (pairs of repo and path)",
        "hypothesis" => "\"cache the parsed config\" (one line)",
        "parent" => "a Kept trial id from history, or omit it",
        "version" => "an integer sealed version, or omit it",
        _ => return generic(last),
    })
}

fn generic(last: &str) -> Option<&'static str> {
    Some(match last {
        "repo" => "\"app\" (a repo name the experiment uses)",
        "sha" | "commit" => "\"68ed4c1a0b7e5f3d9c2a41b6e8f07d35a9c1e2b4\" (a full commit sha)",
        "glob" => "\"bench/**\" (a path pattern; * within a segment, ** across)",
        "path" => "\"bench.js\" (a path in the repo)",
        "sha256" => "a 64-character hex sha256 of the file",
        "name" => "a short string",
        "tolerance" => "5 (a number, zero or more)",
        _ => return None,
    })
}

/// Appends the path of the failing field and a shape example to a parse `error`.
pub fn explain(tool: &str, value: &Value, error: &str) -> String {
    match locate(tool, value) {
        Some(path) => {
            let shown = if path.is_empty() { "the input" } else { &path };
            match example(&path) {
                Some(e) => format!("{error}. The failing field is `{shown}`; its shape is {e}"),
                None => format!("{error}. The failing field is `{shown}`"),
            }
        }
        None => error.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seal() -> Value {
        let at = SEAL_DESCRIPTION.find(r#"{"definition""#).unwrap();
        serde_json::from_str(&SEAL_DESCRIPTION[at..]).unwrap()
    }

    #[test]
    fn the_examples_in_the_descriptions_parse() {
        let parsed: super::super::judge::SealIn = serde_json::from_value(seal()).unwrap();
        for cheat in antilog_types::judge::STANDARD_CHEATS {
            assert!(parsed.cheats.iter().any(|c| c.name == cheat));
        }
        assert!(locate("seal", &seal()).is_none());
        let at = MEASURE_DESCRIPTION.find(r#"{"commits""#).unwrap();
        let m: Value = serde_json::from_str(&MEASURE_DESCRIPTION[at..]).unwrap();
        assert!(serde_json::from_value::<super::super::judge::MeasureIn>(m.clone()).is_ok());
        assert!(locate("measure", &m).is_none());
    }

    fn broken(edit: impl FnOnce(&mut Value)) -> String {
        let mut v = seal();
        edit(&mut v);
        let err = serde_json::from_value::<super::super::judge::SealIn>(v.clone())
            .err()
            .unwrap();
        explain("seal", &v, &err.to_string())
    }

    #[test]
    fn errors_carry_a_path_and_a_shape() {
        let m = broken(|v| v["definition"]["evaluation"]["protected"] = json!(["bench.js"]));
        assert!(m.contains("`definition.evaluation.protected[0]`"), "{m}");
        assert!(m.contains(r#"{"repo":"app","glob""#), "{m}");

        let m = broken(|v| v["definition"]["metric"]["direction"] = json!("lower"));
        assert!(
            m.contains("definition.metric.direction") && m.contains("maximize"),
            "{m}"
        );

        let m = broken(|v| v["definition"]["metric"] = json!("seconds"));
        assert!(m.contains(r#""unit":"s""#), "{m}");

        let m = broken(|v| v["definition"]["data"] = json!([]));
        assert!(
            m.contains("`definition.data`") && m.contains("holdout"),
            "{m}"
        );

        let m = broken(|v| v["definition"]["evaluation"]["repos"][0]["commit"] = json!(7));
        assert!(m.contains("full commit sha"), "{m}");

        let m = broken(|v| v["definition"]["data"]["holdout"]["files"] = json!([]));
        assert!(m.contains("definition.data.holdout.files"), "{m}");

        let m = broken(|v| {
            v["definition"]["metric"]
                .as_object_mut()
                .unwrap()
                .remove("unit");
        });
        assert!(
            m.contains("missing field") && m.contains("definition.metric"),
            "{m}"
        );

        let m = broken(|v| v["definition"]["evaluation"]["command"] = json!(["a"]));
        assert!(m.contains("definition.evaluation.command"), "{m}");
    }

    #[test]
    fn measure_errors_carry_a_shape() {
        let v = json!({"commits":["68ed4c1"],"hypothesis":"x"});
        let err = serde_json::from_value::<super::super::judge::MeasureIn>(v.clone())
            .err()
            .unwrap();
        let m = explain("measure", &v, &err.to_string());
        assert!(m.contains("`commits[0]`") && m.contains(r#""sha""#), "{m}");
    }
}
