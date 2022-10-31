use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct DeploymentInfo {
	pub method: DeploymentMethod,
	pub cpu: u8,
	pub memory: u8,
	// 副本数
	pub replicas: u8,
	// 可用数
	pub acliable: u8,
}

impl DeploymentInfo {
	fn new(method: DeploymentMethod, cpu: u8, memory: u8, replicas: u8, acliable: u8) -> Self {
		DeploymentInfo { method, cpu, memory, replicas, acliable }
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum DeploymentMethod {
	// (image， port）
	Cli(Vec<u8>, u8),
	// ipfs cid
	Ipfs(Vec<u8>),
}
