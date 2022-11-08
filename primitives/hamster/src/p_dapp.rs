use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct DAppInfo<AccountId, BlockNumber> {
	// dapp index
	pub dapp_index: u64,
	// dapp 拥有者
	pub account: AccountId,
	// dapp name
	pub dapp_name: Vec<u8>,
	// 对应的部署方法 index
	pub method_index: u64,
	// 属于哪个资源节点
	pub resource_index: u64,
	// 状态
	pub status: DappStatus,
	// 上一次心跳时间
	pub last_heartbeat: BlockNumber,
}

impl<AccountId, BlockNumber> DAppInfo<AccountId, BlockNumber> {
	pub fn new(
		dapp_index: u64,
		account: AccountId,
		dapp_name: Vec<u8>,
		method_index: u64,
		resource_index: u64,
		status: DappStatus,
		last_heartbeat: BlockNumber,
	) -> Self {
		DAppInfo {
			dapp_index,
			account,
			dapp_name,
			method_index,
			resource_index,
			status,
			last_heartbeat,
		}
	}

	// 重新分配资源节点
	fn _reallocate_resource_node(&self, _resource_index: u64) {}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum DappStatus {
	// 运行
	Online,
	// 暂停服务 (处于升级状态/切换资源节点状态)
	Pause,
	// 销毁
	Destroyed,
}
