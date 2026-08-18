/// Generates the 1:1 string-mapping methods (`as_str`, `as_lower_str`,
/// `from_lower_str`) and the variant list (`all`) from a single canonical
/// table of `Variant => "Name", "lower"`.
///
/// This keeps them impossible to desync (the same `"lower"` literal drives
/// both `as_lower_str` and `from_lower_str`, and the same variant drives
/// `all`), and reduces adding a new adapter to a single line in the table
/// below.
///
/// NOTE: `Custom(_)` and the `genai_` parsing are handled inside the macro since
///       they are constant across all variants. The cfg-gated `BedrockSigv4` is
///       handled directly with `#[cfg]` attributes inside the generated match arms
///       rather than in the table, keeping the table trivially readable.
macro_rules! adapter_kind_str_maps {
	(
		$(
			$variant:ident => $name:literal , $lower:literal , $adapter:ty
		);* $(;)?
	) => {
		impl AdapterKind {
			/// Serialize to a static str
			/// NOTE: Must match case of variant (genai)
			pub fn as_str(&self) -> &'static str {
				match self {
					$( AdapterKind::$variant => $name, )*
					#[cfg(feature = "bedrock-sigv4")]
					AdapterKind::BedrockSigv4 => "BedrockSigv4",
					AdapterKind::Custom(_) => "Custom",
				}
			}

			/// Serialize to a lowercase static str
			pub fn as_lower_str(&self) -> &'static str {
				match self {
					$( AdapterKind::$variant => $lower, )*
					#[cfg(feature = "bedrock-sigv4")]
					AdapterKind::BedrockSigv4 => "bedrock_sigv4",
					AdapterKind::Custom(_) => "custom",
				}
			}

			/// All built-in adapter kinds, in table order.
			///
			/// `Custom(_)` is excluded: it is parameterized by an index
			/// rather than a fixed variant, so it cannot be enumerated.
			///
			/// Useful for building a provider list at runtime — validating
			/// configuration against the known set, or offering a choice —
			/// which `from_lower_str` alone cannot do.
			pub fn all() -> &'static [AdapterKind] {
				&[
					$( AdapterKind::$variant, )*
					#[cfg(feature = "bedrock-sigv4")]
					AdapterKind::BedrockSigv4,
				]
			}

			pub fn from_lower_str(name: &str) -> Option<Self> {
				match name {
					$( $lower => Some(AdapterKind::$variant), )*
					#[cfg(feature = "bedrock-sigv4")]
					"bedrock_sigv4" => Some(AdapterKind::BedrockSigv4),
					name => {
						// Note for now the `genai_` prefix is the way to match to the Custom adapter
						//      This way, namespace, `genai_` ... maps better to the environment variable `GENAI_1_API_KEY`
						if name.starts_with("genai_") {
							name.strip_prefix("genai_")
								.and_then(|n| n.parse::<u8>().ok())
								.map(AdapterKind::Custom)
						} else {
							None
						}
					}
				}
			}

			/// Get the default key environment variable name for the adapter kind (when static)
			pub fn default_key_env_name(&self) -> Option<&'static str> {
				match self {
					$( AdapterKind::$variant => <$adapter>::DEFAULT_API_KEY_ENV_NAME, )*
					#[cfg(feature = "bedrock-sigv4")]
					AdapterKind::BedrockSigv4 => adapters::bedrock::BedrockSigv4Adapter::DEFAULT_API_KEY_ENV_NAME,
					AdapterKind::Custom(_n) => None,
				}
			}
		}
	};
}

pub(crate) use adapter_kind_str_maps;
