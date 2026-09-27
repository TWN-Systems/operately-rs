//! Generates `src/generated.rs` from the vendored `api-catalog.json` (Operately's own
//! machine-readable description of its `/api/external/v1` surface, the same source their
//! official CLI generates its TypeScript client from — see `cli/src/generated/api-catalog.json`
//! in github.com/operately/operately).
//!
//! Re-run automatically whenever `api-catalog.json` changes (see `cargo:rerun-if-changed` below).
//! To pick up a newer Operately release, replace the vendored JSON file and rebuild — nothing
//! here is hand-maintained.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write as _;

fn main() {
    println!("cargo:rerun-if-changed=api-catalog.json");
    println!("cargo:rerun-if-changed=build.rs");

    let raw = std::fs::read_to_string("api-catalog.json").expect("read api-catalog.json");
    let catalog: Value = serde_json::from_str(&raw).expect("parse api-catalog.json");

    let mut out = String::new();
    out.push_str(HEADER);

    let types = &catalog["types"];
    let primitives = types["primitives"].as_object().cloned().unwrap_or_default();
    let enums = types["enums"].as_object().cloned().unwrap_or_default();
    let int_enums = types["int_enums"].as_object().cloned().unwrap_or_default();
    let objects = types["objects"].as_object().cloned().unwrap_or_default();
    let unions = types["unions"].as_object().cloned().unwrap_or_default();

    let names =
        TypeNames { primitives: &primitives, enums: &enums, int_enums: &int_enums, objects: &objects, unions: &unions };

    // Scalars that are not in the catalog's own `types` section at all — built into the
    // wire format itself, per docs/api.md's type list and confirmed against real field usage.
    out.push_str("// ── scalars ─────────────────────────────────────────────────────────\n");
    out.push_str("pub type Id = String;\n");
    out.push_str("pub type OJson = String; // pre-serialized rich-text/document content, not parsed here\n\n");

    out.push_str("// ── enums (string, fixed variant set from the catalog) ─────────────────\n");
    for (name, def) in &enums {
        let variants = def.as_array().expect("enum value is a list");
        emit_string_enum(&mut out, name, variants);
    }
    out.push('\n');

    out.push_str("// ── int enums (kept as plain integers — the catalog gives us the allowed\n");
    out.push_str("// discriminant values as strings, e.g. access levels 0/10/40/70/100, but no\n");
    out.push_str("// semantic label per value, so a fuller enum would just invent names) ─────\n");
    for (name, _) in &int_enums {
        writeln!(out, "pub type {} = i64;", pascal(name)).unwrap();
    }
    out.push('\n');

    out.push_str("// ── unions (deliberately NOT modeled as tagged Rust enums for v1 — each\n");
    out.push_str("// is a large, activity-log-shaped union (dozens of variants) not needed for\n");
    out.push_str("// the tasks/projects/goals/spaces/people surface this crate targets first.\n");
    out.push_str("// Kept as raw JSON so nothing silently drops data.) ─────────────────────\n");
    for (name, _) in &unions {
        writeln!(out, "pub type {} = serde_json::Value;", pascal(name)).unwrap();
    }
    out.push('\n');

    out.push_str("// ── objects ─────────────────────────────────────────────────────────\n");
    // Emit in sorted order for a stable diff between regenerations.
    let mut object_names: Vec<&String> = objects.keys().collect();
    object_names.sort();
    for name in object_names {
        let def = &objects[name];
        let fields = def["fields"].as_array().cloned().unwrap_or_default();
        emit_struct(&mut out, &pascal(name), &fields, &names);
    }
    out.push('\n');

    // ── endpoints, grouped by namespace ─────────────────────────────────────
    let endpoints = catalog["endpoints"].as_array().expect("endpoints is a list");
    let mut namespaces: std::collections::BTreeMap<String, Vec<&Value>> = Default::default();
    for ep in endpoints {
        let ns = ep["namespace"].as_str().unwrap_or("root").to_string();
        namespaces.entry(ns).or_default().push(ep);
    }

    out.push_str("// ── per-endpoint input/output structs and client methods ──────────────\n");
    out.push_str("impl crate::OperatelyClient {\n");
    for (ns, eps) in &namespaces {
        writeln!(out, "    // ─── {ns} ───").unwrap();
        for ep in eps {
            emit_endpoint(&mut out, ns, ep, &names);
        }
    }
    out.push_str("}\n\n");

    // Struct/enum defs for endpoint inputs/outputs are emitted alongside the impl above via
    // a second pass so they land at module top level, not inside the impl block.
    let mut endpoint_types = String::new();
    for (ns, eps) in &namespaces {
        for ep in eps {
            emit_endpoint_types(&mut endpoint_types, ns, ep, &names);
        }
    }
    out.insert_str(out.find("impl crate::OperatelyClient").unwrap(), &endpoint_types);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out_dir}/generated.rs"), out).expect("write generated.rs");
}

const HEADER: &str = "\
// GENERATED — do not hand-edit. See build.rs. Source: operately/operately's api-catalog.json.
use serde::{Deserialize, Serialize};

";

struct TypeNames<'a> {
    primitives: &'a serde_json::Map<String, Value>,
    enums: &'a serde_json::Map<String, Value>,
    int_enums: &'a serde_json::Map<String, Value>,
    objects: &'a serde_json::Map<String, Value>,
    unions: &'a serde_json::Map<String, Value>,
}

fn pascal(snake: &str) -> String {
    snake
        .split(['_', '-'])
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Rust field/variant identifiers can't be a handful of reserved words without the raw-ident
/// `r#` prefix. serde derive round-trips `r#type` as the wire name `"type"` automatically, so no
/// explicit `#[serde(rename = ...)]` is needed for this case specifically.
fn ident(snake: &str) -> String {
    const RESERVED: &[&str] = &[
        "type", "move", "match", "mod", "fn", "let", "ref", "use", "loop", "if", "else", "for", "while", "box", "dyn",
        "self", "super", "in",
    ];
    if RESERVED.contains(&snake) {
        format!("r#{snake}")
    } else {
        snake.to_string()
    }
}

fn resolve_type(ty: &Value, names: &TypeNames) -> String {
    match ty["kind"].as_str() {
        Some("list") => format!("Vec<{}>", resolve_type(&ty["item"], names)),
        _ => {
            let name = ty["name"].as_str().expect("named type has a name");
            resolve_named(name, names)
        }
    }
}

fn resolve_named(name: &str, names: &TypeNames) -> String {
    if names.primitives.contains_key(name) {
        return match name {
            "id" | "company_id" => "Id".to_string(),
            // Confirmed against a real response (GET /projects/list): the "json" scalar is NOT
            // consistently one shape. `description` really is a JSON-encoded string (a TipTap
            // document serialized to text); `tasks_kanban_state` is sent as a genuine nested
            // object, not a string at all. `serde_json::Value` is the only representation that
            // decodes both without guessing which one a given field happens to be - a
            // `Value::String` for the former, `Value::Object` for the latter. `OJson` (a type
            // alias to `String`) is kept only because some already-written input code may
            // reference it; it is no longer used by this generator.
            "json" => "serde_json::Value".to_string(),
            _ => "String".to_string(),
        };
    }
    match name {
        "string" => return "String".to_string(),
        "boolean" => return "bool".to_string(),
        "integer" => return "i64".to_string(),
        "float" => return "f64".to_string(),
        "date" | "datetime" => return "String".to_string(),
        // Confirmed against a real response (GET /projects/list): `timeframe` is
        // `{contextual_start_date: {value, date, date_type}, contextual_end_date: {...}, ...}`,
        // and `contextual_date` is that nested `{value, date, date_type}` shape itself — neither
        // is a plain string despite the catalog naming them alongside "date"/"datetime". Not in
        // the catalog's own `types.objects` map either, so there's no declared field list to
        // generate a real struct from — raw JSON, same reasoning as the union types above.
        "contextual_date" | "timeframe" => return "serde_json::Value".to_string(),
        _ => {}
    }
    if names.enums.contains_key(name) {
        return pascal(name);
    }
    if names.int_enums.contains_key(name) {
        return pascal(name);
    }
    if names.objects.contains_key(name) {
        return pascal(name);
    }
    if names.unions.contains_key(name) {
        return pascal(name);
    }
    // Unknown to the catalog's own type sections (e.g. a task_status/task_type-style dynamic,
    // per-company string that isn't in the fixed `enums` list because it has no fixed variant
    // set) — modeled as a plain String rather than guessing a variant list.
    "String".to_string()
}

/// Any two object types can form a size cycle through each other (confirmed two shapes of
/// this in the real catalog: direct self-reference, `Person.manager: Person`, AND an indirect
/// cycle, `InviteLink.author: Person` + `Person.invite_link: InviteLink`) — rather than detect
/// cycles, every *direct* (non-list) reference to a known object type is boxed, full stop.
/// `Vec<T>` already provides its own heap indirection, so list-kind fields are left alone.
///
/// Every field is wrapped `Option<T>` regardless of the catalog's own `optional`/`nullable`
/// flags. Confirmed against a real `people/list` response (captured live, not synthetic):
/// `person.title` is declared `optional: false, nullable: false` in this exact catalog, and
/// the real server returned `"title": null` for two of three people anyway. The declared
/// flags are not reliable enough to gate deserialization strictness on — being permissive
/// about what a real response can contain matters more here than round-tripping the schema's
/// own (evidently aspirational) optionality claims. Callers constructing *inputs* still get
/// `skip_serializing_if` on `None`, so this costs a `Some(...)` wrapper on the caller's side,
/// not silent data loss or a runtime panic on a null the schema swore couldn't happen.
fn field_type(field: &Value, names: &TypeNames) -> String {
    let ty = &field["type"];
    let is_list = ty["kind"].as_str() == Some("list");
    let inner = resolve_type(ty, names);
    let boxed_inner =
        if !is_list && names.objects.contains_key(bare_catalog_name(ty)) { format!("Box<{inner}>") } else { inner };
    format!("Option<{boxed_inner}>")
}

/// The catalog's own (snake_case) type name for a non-list field, used to check object-ness
/// against `names.objects` (which is keyed by that same snake_case name, not the Pascal Rust
/// name `resolve_type` returns).
fn bare_catalog_name(ty: &Value) -> &str {
    ty["name"].as_str().unwrap_or("")
}

fn emit_struct(out: &mut String, name: &str, fields: &[Value], names: &TypeNames) {
    writeln!(out, "#[derive(Debug, Clone, Serialize, Deserialize, Default)]").unwrap();
    writeln!(out, "pub struct {name} {{").unwrap();
    let mut seen = BTreeSet::new();
    for f in fields {
        let fname = f["name"].as_str().expect("field has a name");
        if fname == "__typename" {
            continue;
        } // GraphQL-era artifact carried into the catalog, not a real field
        if !seen.insert(fname.to_string()) {
            continue;
        } // some object defs repeat a field twice
        let ty = field_type(f, names);
        let is_opt = ty.starts_with("Option<");
        if is_opt {
            writeln!(out, "    #[serde(default, skip_serializing_if = \"Option::is_none\")]").unwrap();
        }
        writeln!(out, "    pub {}: {ty},", ident(fname)).unwrap();
    }
    out.push_str("}\n\n");
}

fn emit_string_enum(out: &mut String, name: &str, variants: &[Value]) {
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]").unwrap();
    writeln!(out, "pub enum {} {{", pascal(name)).unwrap();
    // Explicit per-variant rename with the exact original wire string, not `rename_all` —
    // some variant sets (language codes: "pt-BR", "en") aren't uniformly snake_case, so a
    // blanket case conversion would silently produce the wrong wire value for those.
    //
    // `#[default]` on the first variant is arbitrary (the catalog carries no notion of a
    // "default" enum value) — it exists only so structs containing a required field of this
    // enum type can still derive `Default` for the zero-input-endpoint case. Never rely on
    // this as a meaningful default; always set the real field explicitly.
    for (i, v) in variants.iter().enumerate() {
        let v = v.as_str().expect("enum variant is a string");
        writeln!(out, "    #[serde(rename = {v:?})]").unwrap();
        if i == 0 {
            out.push_str("    #[default]\n");
        }
        writeln!(out, "    {},", pascal(v)).unwrap();
    }
    // The catalog only lists variants that were known when it was generated. A live
    // deployment has sent at least one value outside that set for a real field
    // (Operately's own "not yet decided" success/goal-outcome state serializes as
    // the literal string "nil", not JSON null, on at least one endpoint) - without
    // this, that one unrecognized string fails the ENTIRE response's decode, not
    // just this field. `#[serde(other)]` can't carry the original string (serde's
    // own limitation for C-like string enums), so it's dropped - an acceptable
    // trade-off for "unknown/unset", never for a value the caller needed to act on.
    out.push_str("    #[serde(other)]\n    Unrecognized,\n");
    out.push_str("}\n\n");
}

fn endpoint_type_name(ns: &str, ep_name: &str, suffix: &str) -> String {
    format!("{}{}{}", pascal(ns), pascal(ep_name), suffix)
}

fn emit_endpoint_types(out: &mut String, ns: &str, ep: &Value, names: &TypeNames) {
    let ep_name = ep["name"].as_str().unwrap();
    let inputs = ep["inputs"].as_array().cloned().unwrap_or_default();
    let outputs = ep["outputs"].as_array().cloned().unwrap_or_default();

    let input_name = endpoint_type_name(ns, ep_name, "Input");
    emit_struct(out, &input_name, &inputs, names);

    let output_name = endpoint_type_name(ns, ep_name, "Output");
    emit_struct(out, &output_name, &outputs, names);
}

fn emit_endpoint(out: &mut String, ns: &str, ep: &Value, _names: &TypeNames) {
    let ep_name = ep["name"].as_str().unwrap();
    let path = ep["path"].as_str().unwrap();
    let method = ep["method"].as_str().unwrap_or("POST");
    let docstring = ep["docstring"].as_str().unwrap_or("");
    let input_name = endpoint_type_name(ns, ep_name, "Input");
    let output_name = endpoint_type_name(ns, ep_name, "Output");
    let fn_name = format!("{}_{}", ns, ep_name);

    if !docstring.is_empty() {
        for line in docstring.lines() {
            writeln!(out, "    /// {line}").unwrap();
        }
    }
    writeln!(out, "    /// `{method} {path}`").unwrap();
    writeln!(out, "    pub async fn {fn_name}(&self, input: &{input_name}) -> crate::Result<{output_name}> {{")
        .unwrap();
    if method == "GET" {
        writeln!(out, "        self.query(\"{path}\", input).await").unwrap();
    } else {
        writeln!(out, "        self.mutation(\"{path}\", input).await").unwrap();
    }
    out.push_str("    }\n\n");
}
