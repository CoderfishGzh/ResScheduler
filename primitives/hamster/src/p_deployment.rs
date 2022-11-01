use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct DeploymentInfo<AccountId> {
	pub account: AccountId,
	pub method: DeploymentMethod,
	pub cpu: u8,
	pub memory: u8,
	// 副本数
	pub replicas: u8,
	// 可用数
	pub acliable: u8,
}

impl<AccountId> DeploymentInfo<AccountId> {
	pub fn new(
		account: AccountId,
		method: DeploymentMethod,
		cpu: u8,
		memory: u8,
		replicas: u8,
		acliable: u8,
	) -> Self {
		DeploymentInfo { account, method, cpu, memory, replicas, acliable }
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum DeploymentMethod {
	// (image:port）
	Cli(Vec<u8>),
	// ipfs cid
	Ipfs(Vec<u8>),
}
