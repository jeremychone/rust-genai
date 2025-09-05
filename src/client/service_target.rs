use crate::ModelIden;
use crate::resolver::{AuthData, Endpoint};

/// Service call target.
///
/// Fields:
/// - `endpoint`: Resolved service endpoint.
/// 
/// - `auth`: Authentication data for the request.
/// 
/// - `model`: Target model identifier.
pub struct ServiceTarget {
	pub endpoint: Endpoint,
	pub auth: AuthData,
	pub model: ModelIden,
}
