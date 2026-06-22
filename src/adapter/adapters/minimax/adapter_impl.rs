use crate::adapter::AdapterKind;
use crate::adapter::adapters::anthropic::AnthropicAdapter;
use crate::impl_pass_through_adapter;

pub struct MinimaxAdapter;

impl_pass_through_adapter!(
	name: MinimaxAdapter,
	kind: AdapterKind::MiniMax,
	key_env: Some("MINIMAX_API_KEY"),
	endpoint: "https://api.minimax.io/anthropic/v1/",
	delegate: AnthropicAdapter,
	all_model_names: |_kind, _endpoint, _auth, _web_client| {
		Ok(vec![])
	},
	unsupported: [embeddings],
);
