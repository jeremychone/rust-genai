//! Provider-aware JSON Schema sanitization for structured model output and strict tools.

use serde_json::{Map, Value};

/// Provider schema dialect targeted by [`sanitize_json_schema`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonSchemaDialect {
	/// OpenAI structured response outputs.
	OpenAiStrict,
	/// OpenAI strict function parameters, which require an explicit object root.
	OpenAiStrictTool,
	/// Anthropic JSON outputs and strict tool use.
	AnthropicStructured,
}

/// Sanitize arbitrary JSON Schema for a provider's constrained-decoding dialect.
///
/// This operation is intentionally provider-aware and can be lossy. In particular,
/// OpenAI requires every declared object property to be required, and both providers
/// require `additionalProperties: false` on constrained object schemas. Applying
/// those requirements can narrow the set of JSON instances accepted by the input
/// schema. Provider-accepted annotations and constraints are otherwise preserved;
/// OpenAI `$ref` uses with accepted annotation siblings are inlined because its
/// validator rejects all `$ref` siblings. Unsupported shapes without a useful
/// provider-compatible rewrite are preserved for authoritative backend validation.
pub fn sanitize_json_schema(schema: &Value, dialect: JsonSchemaDialect) -> Value {
	let policy = match dialect {
		JsonSchemaDialect::OpenAiStrict => SchemaPolicy {
			require_all_properties: true,
			inline_ref_annotations: true,
			inline_root_ref: false,
		},
		JsonSchemaDialect::OpenAiStrictTool => SchemaPolicy {
			require_all_properties: true,
			inline_ref_annotations: true,
			inline_root_ref: true,
		},
		JsonSchemaDialect::AnthropicStructured => SchemaPolicy {
			require_all_properties: false,
			inline_ref_annotations: false,
			inline_root_ref: false,
		},
	};
	if policy.inline_root_ref
		&& let Some(inlined) = inline_root_ref(schema)
	{
		normalize_schema(&inlined, &inlined, policy)
	} else {
		normalize_schema(schema, schema, policy)
	}
}

#[derive(Clone, Copy)]
struct SchemaPolicy {
	require_all_properties: bool,
	inline_ref_annotations: bool,
	inline_root_ref: bool,
}

const SCHEMA_MAP_KEYWORDS: &[&str] = &["properties", "patternProperties", "$defs", "definitions", "dependentSchemas"];

const SCHEMA_VALUE_KEYWORDS: &[&str] = &[
	"additionalProperties",
	"unevaluatedProperties",
	"propertyNames",
	"contains",
	"items",
	"additionalItems",
	"unevaluatedItems",
	"not",
	"if",
	"then",
	"else",
	"contentSchema",
];

const SCHEMA_ARRAY_KEYWORDS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];

const REF_INLINE_ANNOTATION_KEYWORDS: &[&str] = &["title", "description", "default", "nullable"];
const ROOT_REF_SIBLING_KEYWORDS: &[&str] = &["$schema", "$defs", "definitions", "title", "description", "default"];

fn inline_root_ref(schema: &Value) -> Option<Value> {
	let map = schema.as_object()?;
	let reference = map.get("$ref")?.as_str()?;
	if !map
		.keys()
		.all(|key| key == "$ref" || ROOT_REF_SIBLING_KEYWORDS.contains(&key.as_str()))
	{
		return None;
	}
	let resolved = resolve_reference(reference, schema)?.as_object()?;
	let mut inlined = resolved.clone();
	for (key, value) in map {
		if key != "$ref" {
			inlined.insert(key.clone(), value.clone());
		}
	}
	Some(Value::Object(inlined))
}

fn normalize_schema(schema: &Value, root: &Value, policy: SchemaPolicy) -> Value {
	match schema {
		Value::Object(map) => normalize_schema_object(map, root, policy),
		other => other.clone(),
	}
}

fn normalize_schema_object(map: &Map<String, Value>, root: &Value, policy: SchemaPolicy) -> Value {
	if policy.inline_ref_annotations
		&& map.len() > 1
		&& let Some(reference) = map.get("$ref").and_then(Value::as_str)
		&& map
			.keys()
			.all(|key| key == "$ref" || REF_INLINE_ANNOTATION_KEYWORDS.contains(&key.as_str()))
		&& let Some(resolved) = resolve_reference(reference, root).and_then(Value::as_object)
	{
		let mut inlined = resolved.clone();
		for (key, value) in map {
			if key != "$ref" {
				inlined.insert(key.clone(), value.clone());
			}
		}
		return normalize_schema(&Value::Object(inlined), root, policy);
	}

	let convert_discriminated_union = map
		.get("oneOf")
		.is_some_and(|one_of| !map.contains_key("anyOf") && one_of_has_disjoint_const_tag(one_of, root));
	let mut out = Map::new();
	for (key, value) in map {
		if convert_discriminated_union && key == "oneOf" {
			out.insert("anyOf".to_string(), normalize_schema_array(value, root, policy));
			continue;
		}
		if SCHEMA_MAP_KEYWORDS.contains(&key.as_str())
			&& let Value::Object(schema_map) = value
		{
			let normalized = schema_map
				.iter()
				.map(|(name, sub_schema)| (name.clone(), normalize_schema(sub_schema, root, policy)))
				.collect();
			out.insert(key.clone(), Value::Object(normalized));
			continue;
		}
		if SCHEMA_VALUE_KEYWORDS.contains(&key.as_str()) {
			out.insert(key.clone(), normalize_schema(value, root, policy));
			continue;
		}
		if SCHEMA_ARRAY_KEYWORDS.contains(&key.as_str()) {
			out.insert(key.clone(), normalize_schema_array(value, root, policy));
			continue;
		}
		out.insert(key.clone(), value.clone());
	}

	let is_object = out.get("type").and_then(Value::as_str) == Some("object") || out.contains_key("properties");
	if is_object {
		out.entry("additionalProperties".to_string()).or_insert(Value::Bool(false));
		if policy.require_all_properties
			&& let Some(Value::Object(properties)) = out.get("properties")
		{
			let mut required: Vec<Value> = properties.keys().cloned().map(Value::String).collect();
			match out.get("required") {
				Some(Value::Array(existing)) if existing.iter().all(Value::is_string) => {
					for name in existing {
						if !required.contains(name) {
							required.push(name.clone());
						}
					}
					out.insert("required".to_string(), Value::Array(required));
				}
				None => {
					out.insert("required".to_string(), Value::Array(required));
				}
				Some(_) => {}
			}
		}
	}

	Value::Object(out)
}

fn normalize_schema_array(value: &Value, root: &Value, policy: SchemaPolicy) -> Value {
	match value {
		Value::Array(items) => Value::Array(items.iter().map(|item| normalize_schema(item, root, policy)).collect()),
		other => other.clone(),
	}
}

/// Return true when every `oneOf` branch requires the same property with a
/// distinct `const` value. Such branches cannot overlap, so replacing
/// `oneOf` with `anyOf` preserves validation semantics. This is the shape
/// schemars emits for internally tagged enums.
fn one_of_has_disjoint_const_tag(one_of: &Value, root: &Value) -> bool {
	let Some(branches) = one_of.as_array().filter(|branches| branches.len() >= 2) else {
		return false;
	};
	let Some(first) = branches
		.first()
		.map(|branch| resolve_local_ref(branch, root))
		.and_then(Value::as_object)
	else {
		return false;
	};
	let Some(first_properties) = first.get("properties").and_then(Value::as_object) else {
		return false;
	};

	first_properties.keys().any(|property_name| {
		let mut seen = Vec::with_capacity(branches.len());
		for branch in branches {
			let Some(branch) = resolve_local_ref(branch, root).as_object() else {
				return false;
			};
			let is_required = branch
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|required| required.iter().any(|name| name.as_str() == Some(property_name)));
			if !is_required {
				return false;
			}
			let Some(constant) = branch
				.get("properties")
				.and_then(Value::as_object)
				.and_then(|properties| properties.get(property_name))
				.and_then(Value::as_object)
				.and_then(|schema| schema.get("const"))
			else {
				return false;
			};
			if seen.contains(constant) {
				return false;
			}
			seen.push(constant.clone());
		}
		true
	})
}

fn resolve_local_ref<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
	let Some(reference) = schema
		.as_object()
		.filter(|map| map.len() == 1)
		.and_then(|map| map.get("$ref"))
		.and_then(Value::as_str)
	else {
		return schema;
	};
	resolve_reference(reference, root).unwrap_or(schema)
}

fn resolve_reference<'a>(reference: &str, root: &'a Value) -> Option<&'a Value> {
	let mut current = root.pointer(reference.strip_prefix('#')?)?;
	for _ in 0..32 {
		let Some(next_reference) = current
			.as_object()
			.filter(|map| map.len() == 1)
			.and_then(|map| map.get("$ref"))
			.and_then(Value::as_str)
		else {
			return Some(current);
		};
		let next = root.pointer(next_reference.strip_prefix('#')?)?;
		if std::ptr::eq(current, next) {
			return None;
		}
		current = next;
	}
	None
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use super::*;
	use schemars::{JsonSchema, schema_for};
	use serde_json::json;

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsItem {
		path: String,
		score: i32,
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	#[schemars(tag = "kind", rename_all = "snake_case")]
	enum SchemarsAnimal {
		Cat { lives: i32 },
		Dog { bark_code: i32 },
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsNode {
		name: String,
		children: Vec<SchemarsNode>,
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsOrdinaryEnvelope {
		results: Vec<SchemarsItem>,
		optional_note: Option<String>,
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsUnionEnvelope {
		animal: SchemarsAnimal,
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsRecursiveEnvelope {
		root: SchemarsNode,
	}

	#[allow(dead_code)]
	#[derive(JsonSchema)]
	struct SchemarsMappingEnvelope {
		lookup: BTreeMap<String, i32>,
	}

	#[test]
	fn openai_strictifies_nested_generated_objects() {
		// OpenAI Responses rejects the equivalent generated schema with:
		// "'required' is required to be supplied and to be an array including
		// every key in properties. Missing '<optional field>'."
		let input = json!({
			"type": "object",
			"properties": {
				"name": {"type": "string"},
				"nested": {
					"type": "object",
					"properties": {"value": {"type": "integer", "default": 3}}
				}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		assert_eq!(result["required"], json!(["name", "nested"]));
		assert_eq!(result["properties"]["nested"]["required"], json!(["value"]));
		assert_eq!(result["properties"]["nested"]["additionalProperties"], json!(false));
		assert_eq!(
			result["properties"]["nested"]["properties"]["value"]["default"],
			json!(3)
		);
	}

	#[test]
	fn openai_sanitizes_live_rejected_generated_fixtures_exactly() {
		let fixture: Value =
			serde_json::from_str(include_str!("../../tests/data/openai_generated_schema_cases.json")).unwrap();
		let cases = fixture["cases"].as_array().unwrap();

		for case in cases {
			let name = case["name"].as_str().unwrap();
			assert!(case["raw_live_error"].as_str().is_some_and(|error| !error.is_empty()));
			let input = &case["input"];
			let expected = &case["expected"];
			assert_ne!(input, expected, "fixture {name} must exercise sanitization");
			assert_eq!(
				sanitize_json_schema(input, JsonSchemaDialect::OpenAiStrict),
				*expected,
				"unexpected OpenAI sanitization for live-rejected fixture {name}"
			);
		}
	}

	#[test]
	fn openai_sanitizes_live_rejected_tool_fixtures_exactly() {
		let fixture: Value =
			serde_json::from_str(include_str!("../../tests/data/openai_generated_tool_schema_cases.json")).unwrap();
		let cases = fixture["cases"].as_array().unwrap();

		for case in cases {
			let name = case["name"].as_str().unwrap();
			assert!(case["raw_live_error"].as_str().is_some_and(|error| !error.is_empty()));
			let input = &case["input"];
			let expected = &case["expected"];
			assert_ne!(input, expected, "fixture {name} must exercise sanitization");
			assert_eq!(
				sanitize_json_schema(input, JsonSchemaDialect::OpenAiStrictTool),
				*expected,
				"unexpected OpenAI strict-tool sanitization for live-rejected fixture {name}"
			);
		}
	}

	#[test]
	fn openai_does_not_drop_existing_required_names() {
		let input = json!({
			"type": "object",
			"properties": {"declared": {"type": "string"}},
			"required": ["external"]
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		assert_eq!(result["required"], json!(["declared", "external"]));
	}

	#[test]
	fn openai_normalizes_discriminated_one_of() {
		// OpenAI Responses rejects this generated shape with:
		// "In context=('properties', 'animal'), 'oneOf' is not permitted."
		let input = json!({
			"type": "object",
			"properties": {
				"animal": {
					"discriminator": {"propertyName": "kind"},
					"oneOf": [{"$ref": "#/$defs/Cat"}, {"$ref": "#/$defs/Dog"}]
				}
			},
			"$defs": {
				"Cat": {
					"type": "object",
					"properties": {"kind": {"const": "cat"}},
					"required": ["kind"]
				},
				"Dog": {
					"type": "object",
					"properties": {"kind": {"const": "dog"}},
					"required": ["kind"]
				}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		let animal = &result["properties"]["animal"];
		assert!(animal.get("oneOf").is_none());
		assert_eq!(animal["discriminator"], json!({"propertyName": "kind"}));
		assert_eq!(
			animal["anyOf"],
			json!([{"$ref": "#/$defs/Cat"}, {"$ref": "#/$defs/Dog"}])
		);
	}

	#[test]
	fn openai_sanitizes_schemars_generated_schemas() {
		let ordinary = serde_json::to_value(schema_for!(SchemarsOrdinaryEnvelope)).unwrap();
		let union = serde_json::to_value(schema_for!(SchemarsUnionEnvelope)).unwrap();
		let recursive = serde_json::to_value(schema_for!(SchemarsRecursiveEnvelope)).unwrap();
		let mapping = serde_json::to_value(schema_for!(SchemarsMappingEnvelope)).unwrap();
		let root_array = serde_json::to_value(schema_for!(Vec<SchemarsItem>)).unwrap();

		// Raw schemars object/recursive schemas are rejected with:
		// "'additionalProperties' is required to be supplied and to be false."
		let ordinary = sanitize_json_schema(&ordinary, JsonSchemaDialect::OpenAiStrict);
		let recursive = sanitize_json_schema(&recursive, JsonSchemaDialect::OpenAiStrict);
		assert_eq!(ordinary["additionalProperties"], json!(false));
		assert_eq!(ordinary["required"], json!(["results", "optional_note"]));
		assert_eq!(recursive["$defs"]["SchemarsNode"]["additionalProperties"], json!(false));

		// Raw schemars internally tagged enums are rejected with:
		// "In context=(), 'oneOf' is not permitted." Their required `kind`
		// constants are pairwise disjoint, making `anyOf` equivalent.
		let union = sanitize_json_schema(&union, JsonSchemaDialect::OpenAiStrict);
		let animal = &union["$defs"]["SchemarsAnimal"];
		assert!(animal.get("oneOf").is_none());
		assert_eq!(animal["anyOf"].as_array().map(Vec::len), Some(2));

		// There is no equivalent strict-schema representation for dynamic maps
		// or root arrays, so their defining shapes remain present for provider
		// validation rather than being silently rewritten.
		let mapping = sanitize_json_schema(&mapping, JsonSchemaDialect::OpenAiStrict);
		let root_array = sanitize_json_schema(&root_array, JsonSchemaDialect::OpenAiStrict);
		assert!(mapping["properties"]["lookup"]["additionalProperties"].is_object());
		assert_eq!(root_array["type"], json!("array"));
	}

	#[test]
	fn openai_preserves_property_names_and_semantic_constraints() {
		let input = json!({
			"type": "object",
			"properties": {
				"pattern": {
					"type": "string",
					"pattern": "^[a-z]+$",
					"minLength": 2
				},
				"default": {"type": "string"},
				"strict": {"type": "boolean"},
				"additionalProperties": {"type": "string"}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		assert!(result["properties"].get("pattern").is_some());
		assert!(result["properties"].get("default").is_some());
		assert!(result["properties"].get("strict").is_some());
		assert!(result["properties"].get("additionalProperties").is_some());
		assert_eq!(result["properties"]["pattern"]["pattern"], json!("^[a-z]+$"));
		assert_eq!(result["properties"]["pattern"]["minLength"], json!(2));
	}

	#[test]
	fn openai_leaves_live_accepted_keywords_untouched() {
		let input = json!({
			"type": "object",
			"properties": {
				"annotated": {
					"type": "string",
					"title": "Annotated",
					"description": "Useful guidance",
					"default": "fallback",
					"nullable": true
				},
				"constrained": {
					"type": "string",
					"minLength": 2,
					"pattern": "^[a-z]+$",
					"format": "email"
				},
				"number": {"type": "integer", "minimum": 1}
			},
			"required": ["annotated", "constrained", "number"],
			"additionalProperties": false
		});
		assert_eq!(sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict), input);
	}

	#[test]
	fn openai_preserves_bare_one_of_for_backend_validation() {
		// OpenAI Responses returns "'oneOf' is not permitted", but these
		// branches are not provably disjoint, so changing it would be unsafe.
		let input = json!({
			"oneOf": [
				{"type": "string", "minLength": 2},
				{"type": "integer", "minimum": 1}
			]
		});
		assert_eq!(sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict), input);
	}

	#[test]
	fn openai_preserves_one_of_when_const_tags_do_not_prove_disjointness() {
		let duplicate_tag = json!({
			"oneOf": [
				{
					"type": "object",
					"properties": {"kind": {"const": "same"}, "a": {"type": "string"}},
					"required": ["kind", "a"]
				},
				{
					"type": "object",
					"properties": {"kind": {"const": "same"}, "b": {"type": "string"}},
					"required": ["kind", "b"]
				}
			]
		});
		let optional_tag = json!({
			"oneOf": [
				{
					"type": "object",
					"properties": {"kind": {"const": "a"}}
				},
				{
					"type": "object",
					"properties": {"kind": {"const": "b"}}
				}
			]
		});

		for input in [duplicate_tag, optional_tag] {
			let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
			assert!(result.get("oneOf").is_some());
			assert!(result.get("anyOf").is_none());
		}
	}

	#[test]
	fn openai_preserves_discriminator_when_referenced_branches_can_overlap() {
		let input = json!({
			"type": "object",
			"properties": {
				"animal": {
					"discriminator": {"propertyName": "kind"},
					"oneOf": [{"$ref": "#/$defs/Cat"}, {"$ref": "#/$defs/Dog"}]
				}
			},
			"$defs": {
				"Cat": {"type": "object", "properties": {"kind": {"const": "pet"}}},
				"Dog": {"type": "object", "properties": {"kind": {"const": "pet"}}}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		let animal = &result["properties"]["animal"];
		assert!(animal.get("oneOf").is_some());
		assert!(animal.get("anyOf").is_none());
		assert!(animal.get("discriminator").is_some());
	}

	#[test]
	fn anthropic_preserves_optional_properties_and_defaults() {
		let input = json!({
			"type": "object",
			"properties": {
				"required_name": {"type": "string"},
				"optional_limit": {"type": "integer", "default": 10}
			},
			"required": ["required_name"]
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::AnthropicStructured);
		assert_eq!(
			result,
			json!({
				"type": "object",
				"properties": {
					"required_name": {"type": "string"},
					"optional_limit": {"type": "integer", "default": 10}
				},
				"required": ["required_name"],
				"additionalProperties": false
			})
		);
	}

	#[test]
	fn openai_inlines_supported_ref_annotations_without_dropping_them() {
		let input = json!({
			"type": "object",
			"properties": {
				"annotated": {
					"$ref": "#/$defs/Name",
					"description": "display name",
					"default": "unknown"
				},
				"constrained": {
					"$ref": "#/$defs/Name",
					"minLength": 2
				},
				"bare": {"$ref": "#/$defs/Name"}
			},
			"$defs": {"Name": {"type": "string", "title": "Name"}}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);

		assert_eq!(
			result["properties"]["annotated"],
			json!({
				"type": "string",
				"title": "Name",
				"description": "display name",
				"default": "unknown"
			})
		);
		assert_eq!(result["properties"]["constrained"], input["properties"]["constrained"]);
		assert_eq!(result["properties"]["bare"], input["properties"]["bare"]);
	}

	#[test]
	fn schema_sanitization_does_not_rewrite_literal_objects() {
		let input = json!({
			"type": "object",
			"properties": {
				"payload": {
					"const": {
						"type": "object",
						"properties": {"not_a_schema": true}
					}
				}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		assert_eq!(
			result["properties"]["payload"]["const"],
			input["properties"]["payload"]["const"]
		);
	}

	#[test]
	fn openai_dynamic_map_is_preserved_for_backend_validation() {
		// OpenAI Responses rejects schema-valued `additionalProperties`; there
		// is no equivalent strict-schema rewrite for `dict<String, i32>`.
		let input = json!({
			"type": "object",
			"properties": {
				"lookup": {"type": "object", "additionalProperties": {"type": "integer"}}
			}
		});
		let result = sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict);
		assert_eq!(
			result["properties"]["lookup"]["additionalProperties"],
			json!({"type": "integer"})
		);
		assert_eq!(result["required"], json!(["lookup"]));
	}

	#[test]
	fn openai_top_level_array_is_preserved_for_backend_validation() {
		// OpenAI Responses returns: "schema must be a JSON Schema of
		// 'type: \"object\"', got 'type: \"array\"'."
		let input = json!({"type": "array", "items": {"type": "integer"}});
		assert_eq!(sanitize_json_schema(&input, JsonSchemaDialect::OpenAiStrict), input);
	}
}
