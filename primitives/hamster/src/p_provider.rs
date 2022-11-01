use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

/// ComputingResources
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ComputingResource<AccountId> {
	// 资源索引
	pub index: u64,
	// 资源拥有者
	pub account_id: AccountId,
	// 指定资源p2p身份
	pub peer_id: Vec<u8>,
	// 资源公网ip
	pub public_ip: Vec<u8>,
	// 资源状态
	pub config: ResourceConfig,
	// 构建的服务组,记录 dapp index 组
	// 如果该资源挂掉，需要将所有服务转移到另外一个资源节点
	pub dapps: Vec<u64>,
	// 资源在线状态
	pub status: ResourceStatus,
}

impl<AccountId> ComputingResource<AccountId> {
	pub fn new(
		index: u64,
		account_id: AccountId,
		peer_id: Vec<u8>,
		public_ip: Vec<u8>,
		config: ResourceConfig,
		dapps: Vec<u64>,
		status: ResourceStatus,
	) -> Self {
		ComputingResource { index, account_id, peer_id, public_ip, config, dapps, status }
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ResourceStatus {
	// 在线
	Online,
	// 宕机
	Offline,
}

impl Default for ResourceStatus {
	fn default() -> Self {
		ResourceStatus::Online
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ResourceConfig {
	// 总资源
	pub total_cpu: u8,
	pub total_memory: u8,
	// 剩下未使用的资源
	pub unused_cpu: u8,
	pub unused_memory: u8,
}

impl ResourceConfig {
	pub fn new(total_cpu: u8, total_memory: u8) -> Self {
		ResourceConfig { total_cpu, total_memory, unused_cpu: 0, unused_memory: 0 }
	}

	// 使用资源
	pub fn use_resource(&mut self, cpu: u8, memory: u8) -> bool {
		if cpu > self.unused_cpu || memory > self.unused_memory {
			return false
		}
		self.unused_cpu = self.unused_cpu.saturating_sub(cpu);
		self.unused_memory = self.unused_memory.saturating_sub(memory);

		true
	}

	// 释放资源
	fn _release_resource(&self, _cpu: u8, _memory: u8) -> bool {
		false
	}

	// 增加DApp
	fn _add_dapp(&self, _dapp_index: u64) -> bool {
		false
	}

	// 删除DApp
	fn _remove_dapp(&self, _dapp_index: u64) -> bool {
		false
	}
}
